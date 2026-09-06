//! Regressions for trait contracts.

use super::*;

#[test]
fn persistence_inventory_unions_cfg_alternative_trait_signatures() {
    let baseline = inventory(
        r#"
                struct Holder(wow_database::CharacterDatabase);

                #[cfg(feature = "database")]
                trait Maker { fn make(&self) -> Holder; }

                #[cfg(not(feature = "database"))]
                trait Maker { fn make(&self) -> u8; }

                #[cfg(feature = "database")]
                fn persistent(factory: &dyn Maker) {
                    consume(factory.make().0.pool());
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
fn persistence_inventory_analyzes_default_trait_method_bodies() {
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
    let consumer = ClassifiedPersistenceSource {
        classification: "database_adapter_core",
        package: "fixture",
        module: "crate::consumer",
        source_path: "src/consumer.rs",
        inherited_cfg: &[],
        source: r#"
                use crate::dto::Holder;
                trait Access {
                    fn holder(&self) -> Holder;
                    fn leak(&self) {
                        consume(self.holder().0.pool());
                    }
                }
            "#,
    };

    let baseline = inventory_persistence_accesses(&[consumer, dto]).unwrap();
    assert!(baseline.accesses.iter().any(|row| {
        row.enclosing == "trait Access::leak"
            && row.target == PersistenceTarget::CharacterDatabase
            && row.operation == PersistenceOperation::PoolAccess
    }));
}

#[test]
fn persistence_inventory_substitutes_impl_associated_type_returns() {
    let baseline = inventory(
        r#"
                struct Holder(wow_database::CharacterDatabase);
                struct PlainHolder(u8);
                trait Maker {
                    type Product;
                    fn make(&self) -> Self::Product;
                }
                struct Factory;
                impl Maker for Factory {
                    type Product = Holder;
                    fn make(&self) -> Self::Product { unreachable!() }
                }
                struct PlainFactory;
                impl Maker for PlainFactory {
                    type Product = PlainHolder;
                    fn make(&self) -> Self::Product { unreachable!() }
                }
                fn persistent(factory: &Factory) { consume(factory.make().0.pool()); }
                fn clean(factory: &PlainFactory) { consume(factory.make().0); }
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
fn persistence_inventory_applies_associated_bindings_to_inherited_default_methods() {
    let baseline = inventory(
        r#"
                struct Holder(wow_database::CharacterDatabase);
                trait Maker {
                    type Product;
                    fn make(&self) -> Self::Product { unreachable!() }
                }
                struct Factory;
                impl Maker for Factory { type Product = Holder; }
                fn persistent(factory: &Factory) { consume(factory.make().0.pool()); }
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
fn persistence_inventory_resolves_self_constants_through_the_active_trait() {
    let baseline = inventory(
        r#"
                trait Sql {
                    const SQL: &'static str;
                    const FALLBACK_SQL: &'static str = "SELECT GET_LOCK('trait-default', 0)";
                }
                struct Statements;
                impl Statements {
                    const SQL: &'static str = "SELECT 1";
                }
                impl Sql for Statements {
                    const SQL: &'static str = "SELECT GET_LOCK('impl-override', 0)";
                    fn overridden(&self) {
                        sqlx::query(Self::SQL);
                    }
                    fn defaulted(&self) {
                        sqlx::query(Self::FALLBACK_SQL);
                    }
                }
                impl Statements {
                    fn inherent(&self) {
                        sqlx::query(Self::SQL);
                    }
                }
            "#,
    )
    .unwrap();
    // The trait impl's own constant wins over the inherent one of the same
    // name; before #204 the inherent key was the only one consulted, so this
    // advisory lock was invisible and its fingerprint never moved.
    assert!(
        baseline.accesses.iter().any(|row| {
            row.enclosing.contains("overridden")
                && row.operation == PersistenceOperation::AdvisoryLock
        }),
        "a trait impl's Self::SQL lost its advisory identity"
    );
    // A trait default is reachable through Self as well.
    assert!(
        baseline.accesses.iter().any(|row| {
            row.enclosing.contains("defaulted")
                && row.operation == PersistenceOperation::AdvisoryLock
        }),
        "a trait default constant was not reachable through Self"
    );
    // The inherent impl still resolves to the inherent constant, which is
    // not a lock: trait resolution must not leak across impls.
    assert!(
        !baseline.accesses.iter().any(|row| {
            row.enclosing.contains("inherent")
                && row.operation == PersistenceOperation::AdvisoryLock
        }),
        "an inherent Self::SQL was classified from a trait impl's constant"
    );
}

#[test]
fn persistence_inventory_pins_trait_default_and_self_constants() {
    let collect = |sql: &str| {
        inventory(&format!(
            r#"
                    trait Sql {{
                        const SQL: &'static str = {sql:?};
                    }}
                    struct Statements;
                    impl Statements {{
                        const OWN: &str = {sql:?};
                        fn run(&self) {{
                            sqlx::query(Self::OWN);
                        }}
                    }}
                    fn from_default() {{
                        sqlx::query(<Statements as Sql>::SQL);
                    }}
                "#
        ))
        .unwrap()
    };
    let locked = collect("SELECT GET_LOCK('default', 0)");
    let clean = collect("SELECT 1");
    let fingerprint = |baseline: &PersistenceAccessBaseline, needle: &str| {
        baseline
            .accesses
            .iter()
            .find(|row| {
                row.enclosing.contains(needle) && row.operation == PersistenceOperation::Query
            })
            .map(|row| row.fingerprint.clone())
            .unwrap()
    };
    // `Self::OWN` names the impl's own constant, and a trait's default is
    // the value an impl inherits.
    for needle in ["run", "from_default"] {
        assert_ne!(
            fingerprint(&locked, needle),
            fingerprint(&clean, needle),
            "{needle} did not follow the constant it names"
        );
    }
}

#[test]
fn persistence_inventory_filters_cfg_on_trait_default_constants() {
    let baseline = inventory(
        r#"
                trait Sql {
                    #[cfg(not(test))]
                    const SQL: &'static str = "SELECT 1";
                    #[cfg(test)]
                    const SQL: &'static str = "SELECT GET_LOCK('test-only', 0)";
                }
                struct Statements;
                impl Sql for Statements {}
                fn production() {
                    sqlx::query(<Statements as Sql>::SQL);
                }
            "#,
    )
    .unwrap();
    // A `cfg(test)` default must not reach the production view, or the
    // frozen baseline gains a statement the server never runs and every
    // edit to test SQL churns production rows.
    assert!(!baseline.accesses.iter().any(|row| {
        row.source_class == "production" && row.operation == PersistenceOperation::AdvisoryLock
    }));
}

#[test]
fn persistence_inventory_classifies_trait_ufcs_from_receiver_flow() {
    let baseline = inventory(
        r#"
                trait PoolProvider { fn pool(&self); }
                fn persistent(database: wow_database::CharacterDatabase) {
                    PoolProvider::pool(&database);
                }
            "#,
    )
    .unwrap();
    assert!(baseline.accesses.iter().any(|row| {
        row.enclosing == "fn persistent"
            && row.operation == PersistenceOperation::PoolAccess
            && row.target == PersistenceTarget::CharacterDatabase
    }));
}

#[test]
fn persistence_inventory_distinguishes_same_named_trait_impl_workflows() {
    let baseline = inventory(
        r#"
                trait LoginStore { fn save(&self); }
                trait CharacterStore { fn save(&self); }
                struct Worker {
                    login: wow_database::LoginDatabase,
                    character: wow_database::CharacterDatabase,
                }
                impl LoginStore for Worker {
                    fn save(&self) { consume(self.login.pool()); }
                }
                impl CharacterStore for Worker {
                    fn save(&self) { consume(self.character.pool()); }
                }
            "#,
    )
    .unwrap();
    let workflows = baseline
        .accesses
        .iter()
        .filter(|row| row.operation == PersistenceOperation::PoolAccess)
        .map(|row| row.enclosing.clone())
        .collect::<BTreeSet<_>>();
    assert!(
        workflows
            .iter()
            .any(|name| name.contains("LoginStore for Worker::save"))
    );
    assert!(
        workflows
            .iter()
            .any(|name| name.contains("CharacterStore for Worker::save"))
    );
}

#[test]
fn persistence_inventory_registers_default_trait_method_parameters() {
    let baseline = inventory(
        r#"
                struct Holder(wow_database::CharacterDatabase);
                trait Access {
                    fn leak(&self, holder: Holder) { consume(holder.0.pool()); }
                }
            "#,
    )
    .unwrap();
    assert!(
        baseline
            .accesses
            .iter()
            .any(|row| row.enclosing == "trait Access::leak"
                && row.operation == PersistenceOperation::PoolAccess)
    );
}

#[test]
fn persistence_inventory_canonicalizes_qualified_inherent_impl_owners() {
    let dto = ClassifiedPersistenceSource {
        classification: "database_adapter_core",
        package: "fixture",
        module: "crate::dto",
        source_path: "src/dto.rs",
        inherited_cfg: &[],
        source: "struct Holder(wow_database::CharacterDatabase); struct Factory;",
    };
    let extensions = ClassifiedPersistenceSource {
        classification: "database_adapter_core",
        package: "fixture",
        module: "crate::extensions",
        source_path: "src/extensions.rs",
        inherited_cfg: &[],
        source: "impl crate::dto::Factory { fn make(&self) -> crate::dto::Holder { unreachable!() } }",
    };
    let consumer = ClassifiedPersistenceSource {
        classification: "database_adapter_core",
        package: "fixture",
        module: "crate::consumer",
        source_path: "src/consumer.rs",
        inherited_cfg: &[],
        source: "fn persistent(factory: crate::dto::Factory) { consume(factory.make().0.pool()); }",
    };
    let baseline = inventory_persistence_accesses(&[consumer, dto, extensions]).unwrap();
    assert!(
        baseline
            .accesses
            .iter()
            .any(|row| row.enclosing == "fn persistent"
                && row.operation == PersistenceOperation::PoolAccess)
    );
}
