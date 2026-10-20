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

/// The F5 host-trait shapes from #1437: persistence-shaped names on receivers
/// the analyzer types without the function's type parameters, and a call to a
/// module-qualified workspace free function, are audited like any other call.
#[test]
fn persistence_inventory_accepts_parameter_free_receivers_in_generic_functions() {
    let result = inventory(
        r#"
                #[derive(Default)]
                pub struct Outcome { pub actions: Vec<u32> }
                impl Outcome { pub fn append(&mut self, other: Outcome) { let _ = other; } }
                pub struct Finalization;
                impl Finalization { pub fn begin(&mut self, step: u8) -> bool { step > 0 } }
                pub struct Lifecycle { finalization: Option<Finalization> }
                impl Lifecycle {
                    pub fn finalization_mut(&mut self) -> Option<&mut Finalization> {
                        self.finalization.as_mut()
                    }
                }
                pub trait Host {
                    fn read(&self) -> u32;
                    fn lifecycle_mut(&mut self) -> &mut Lifecycle;
                }
                pub mod player_cast {
                    pub fn prepare<H: super::Host>(host: &mut H) -> bool { host.read() > 0 }
                }
                pub fn apply<H: Host>(host: &mut H) -> Outcome {
                    let mut outcome = Outcome::default();
                    let mut list = Vec::new();
                    let mut more = vec![host.read()];
                    list.append(&mut more);
                    outcome.actions.append(&mut list);
                    outcome.append(Outcome::default());
                    if host.lifecycle_mut().finalization_mut().unwrap().begin(1) {}
                    if host.lifecycle_mut().finalization_mut().expect("x").begin(2) {}
                    if crate::player_cast::prepare(host) {}
                    outcome
                }
            "#,
    );
    assert!(result.is_ok(), "{result:?}");
}

/// Receivers that may still be a type parameter keep failing (#1437).
#[test]
fn persistence_inventory_rejects_parameter_dependent_receivers_in_generic_functions() {
    let rejected = |body: &str, operation: &str| {
        let source = format!(
            r#"
                pub trait HasPool {{ fn pool(&self); fn begin(&self) -> bool; }}
                pub struct Holder<T>(pub T);
                pub struct Wrapper<T> {{ pub inner: T }}
                impl<T> Wrapper<T> {{ pub fn begin(&self) -> bool {{ true }} }}
                pub trait Host {{ type Db: HasPool; fn db(&mut self) -> &mut Self::Db; }}
                pub trait Exec {{ fn execute(&self); }}
                {body}
            "#
        );
        let error = inventory(&source).unwrap_err();
        assert!(
            error.contains(&format!("persistence-shaped operations ({operation})")),
            "{body}: {error}"
        );
    };
    rejected(
        "pub fn f<T: HasPool>(value: T) { let held = Holder(value); held.0.pool(); }",
        "pool",
    );
    rejected("pub fn f<T>(value: Wrapper<T>) { value.begin(); }", "begin");
    rejected(
        "pub fn f<T: HasPool>(value: Option<T>) { value.unwrap().pool(); }",
        "pool",
    );
    rejected(
        "pub fn f<H: Host>(host: &mut H) { host.db().begin(); }",
        "begin",
    );
    rejected(
        "pub fn f<T: Exec>(value: T) { Exec::execute(&value); }",
        "execute",
    );
    rejected(
        "pub fn f<T: HasPool>(value: T) { unknown_root::prepare(value); }",
        "prepare",
    );
    rejected(
        "pub fn f<T: Exec>(value: T) { crate::Exec::execute(&value); }",
        "execute",
    );
}

/// Across packages (#1437): a dependency's free function and declared type
/// are parameter-free, while the dependency's own generic body is still held
/// to the check at its definition, and an undeclared root is not trusted.
#[test]
fn persistence_inventory_audits_dependency_free_functions_at_their_definition() {
    let source = |package, source| ClassifiedPersistenceSource {
        classification: "database_adapter_core",
        package,
        module: "crate",
        source_path: "src/lib.rs",
        inherited_cfg: &[],
        source,
    };
    let provider = source(
        "provider-a",
        r#"
            pub trait Host { fn lifecycle_mut(&mut self) -> &mut Lifecycle; }
            pub struct Lifecycle;
            impl Lifecycle { pub fn begin(&mut self) -> bool { true } }
            pub mod player_cast {
                pub fn prepare<H: super::Host>(host: &mut H) -> bool {
                    host.lifecycle_mut().begin()
                }
            }
        "#,
    );
    let consumer = source(
        "consumer-b",
        r#"
            pub fn tick<H: provider_alias::Host>(host: &mut H) -> bool {
                provider_alias::player_cast::prepare(host) && host.lifecycle_mut().begin()
            }
        "#,
    );
    // Test builds see the normal dependencies too, as Cargo does.
    let aliases = BTreeMap::from([(
        "consumer-b".to_owned(),
        BTreeMap::from([("provider_alias".to_owned(), "provider_a".to_owned())]),
    )]);
    let dependencies = WorkspaceDependencyAliases {
        production: aliases.clone(),
        test: aliases,
    };
    let accepted =
        inventory_persistence_accesses_with_dependencies(&[consumer, provider], &dependencies);
    assert!(accepted.is_ok(), "{accepted:?}");

    let generic_provider = source(
        "provider-a",
        r#"
            pub trait HasPool { fn pool(&self); }
            pub mod player_cast {
                pub fn prepare<T: super::HasPool>(value: T) { value.pool(); }
            }
        "#,
    );
    let generic_consumer = source(
        "consumer-b",
        r#"
            pub fn tick<T: provider_alias::HasPool>(value: T) {
                provider_alias::player_cast::prepare(value);
            }
            pub fn untrusted<T: provider_alias::HasPool>(value: T) {
                unrelated_provider::player_cast::prepare(value);
            }
        "#,
    );
    let error = inventory_persistence_accesses_with_dependencies(
        &[generic_consumer, generic_provider],
        &dependencies,
    )
    .unwrap_err();
    assert!(
        error.contains("fn prepare is generic and contains persistence-shaped operations (pool)"),
        "{error}"
    );
    assert!(
        error.contains(
            "fn untrusted is generic and contains persistence-shaped operations (prepare)"
        ),
        "{error}"
    );
    assert!(!error.contains("fn tick "), "{error}");
}
