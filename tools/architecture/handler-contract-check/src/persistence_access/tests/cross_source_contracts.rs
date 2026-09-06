//! Regressions for cross source contracts.

use super::*;

#[test]
fn persistence_inventory_resolves_trait_signatures_across_source_files() {
    let dto = ClassifiedPersistenceSource {
        classification: "database_adapter_core",
        package: "fixture",
        module: "crate::dto",
        source_path: "src/dto.rs",
        inherited_cfg: &[],
        source: r#"
                pub type Db = wow_database::CharacterDatabase;
                pub struct Holder(pub Db);
                pub struct Plain(pub u8);
            "#,
    };
    let declaration = ClassifiedPersistenceSource {
        classification: "database_adapter_core",
        package: "fixture",
        module: "crate::maker",
        source_path: "src/maker.rs",
        inherited_cfg: &[],
        source: r#"
                use crate::dto::{Holder, Plain};
                pub trait Maker { fn make(&self) -> Holder; }
                pub trait PlainMaker { fn make(&self) -> Plain; }
            "#,
    };
    let consumer = ClassifiedPersistenceSource {
        classification: "database_adapter_core",
        package: "fixture",
        module: "crate::consumer",
        source_path: "src/consumer.rs",
        inherited_cfg: &[],
        source: r#"
                fn cross_file_persistent(factory: &dyn crate::maker::Maker) {
                    consume(factory.make().0.pool());
                }
                fn cross_file_clean(factory: &dyn crate::maker::PlainMaker) {
                    consume(factory.make().0);
                }
            "#,
    };

    let baseline = inventory_persistence_accesses(&[consumer, declaration, dto]).unwrap();
    assert!(baseline.accesses.iter().any(|row| {
        row.enclosing == "fn cross_file_persistent"
            && row.target == PersistenceTarget::CharacterDatabase
            && row.operation == PersistenceOperation::PoolAccess
    }));
    assert!(
        !baseline
            .accesses
            .iter()
            .any(|row| row.enclosing == "fn cross_file_clean")
    );
}

#[test]
fn persistence_inventory_registers_enum_variant_payloads_across_source_files() {
    let dto = ClassifiedPersistenceSource {
        classification: "database_adapter_core",
        package: "fixture",
        module: "crate::dto",
        source_path: "src/dto.rs",
        inherited_cfg: &[],
        source: r#"
                pub type Db = wow_database::CharacterDatabase;
                pub enum Product { Database(Db), Plain(u8) }
            "#,
    };
    let declaration = ClassifiedPersistenceSource {
        classification: "database_adapter_core",
        package: "fixture",
        module: "crate::maker",
        source_path: "src/maker.rs",
        inherited_cfg: &[],
        source: r#"
                pub trait Maker { fn make(&self) -> crate::dto::Product; }
            "#,
    };
    let consumer = ClassifiedPersistenceSource {
        classification: "database_adapter_core",
        package: "fixture",
        module: "crate::consumer",
        source_path: "src/consumer.rs",
        inherited_cfg: &[],
        source: r#"
                fn cross_file_persistent(factory: &dyn crate::maker::Maker) {
                    match factory.make() {
                        crate::dto::Product::Database(database) => consume(database.pool()),
                        crate::dto::Product::Plain(_) => {}
                    }
                }
            "#,
    };

    let baseline = inventory_persistence_accesses(&[consumer, declaration, dto]).unwrap();
    assert!(baseline.accesses.iter().any(|row| {
        row.enclosing == "fn cross_file_persistent"
            && row.target == PersistenceTarget::CharacterDatabase
            && row.operation == PersistenceOperation::PoolAccess
    }));
}

#[test]
fn persistence_inventory_resolves_free_functions_across_source_files() {
    let factory = ClassifiedPersistenceSource {
        classification: "database_adapter_core",
        package: "fixture",
        module: "crate::factory",
        source_path: "src/factory.rs",
        inherited_cfg: &[],
        source: r#"
                pub fn database() -> wow_database::CharacterDatabase {
                    unimplemented!()
                }
            "#,
    };
    let qualified = ClassifiedPersistenceSource {
        classification: "database_adapter_core",
        package: "fixture",
        module: "crate::qualified",
        source_path: "src/qualified.rs",
        inherited_cfg: &[],
        source: r#"
                fn persistent_qualified() {
                    consume(crate::factory::database().pool());
                }
            "#,
    };
    let imported = ClassifiedPersistenceSource {
        classification: "database_adapter_core",
        package: "fixture",
        module: "crate::imported",
        source_path: "src/imported.rs",
        inherited_cfg: &[],
        source: r#"
                use crate::factory::database;
                fn persistent_imported() {
                    consume(database().pool());
                }
            "#,
    };

    let baseline = inventory_persistence_accesses(&[qualified, imported, factory]).unwrap();
    assert!(baseline.accesses.iter().any(|row| {
        row.enclosing == "fn persistent_qualified"
            && row.target == PersistenceTarget::CharacterDatabase
            && row.operation == PersistenceOperation::PoolAccess
    }));
    assert!(baseline.accesses.iter().any(|row| {
        row.enclosing == "fn persistent_imported"
            && row.target == PersistenceTarget::CharacterDatabase
            && row.operation == PersistenceOperation::PoolAccess
    }));
}

#[test]
fn persistence_inventory_resolves_inherent_methods_across_source_files() {
    let dto = ClassifiedPersistenceSource {
        classification: "database_adapter_core",
        package: "fixture",
        module: "crate::dto",
        source_path: "src/dto.rs",
        inherited_cfg: &[],
        source: r#"
                pub type Db = wow_database::CharacterDatabase;
                pub struct Factory;
                impl Factory { pub fn make(&self) -> Db { unreachable!() } }
            "#,
    };
    let consumer = ClassifiedPersistenceSource {
        classification: "database_adapter_core",
        package: "fixture",
        module: "crate::consumer",
        source_path: "src/consumer.rs",
        inherited_cfg: &[],
        source: r#"
                use crate::dto::Factory;
                fn persistent(factory: &Factory) {
                    consume(factory.make().pool());
                }
                fn persistent_qualified(factory: &Factory) {
                    consume(crate::dto::Factory::make(factory).pool());
                }
            "#,
    };

    let baseline = inventory_persistence_accesses(&[consumer, dto]).unwrap();
    for enclosing in ["fn persistent", "fn persistent_qualified"] {
        assert!(
            baseline.accesses.iter().any(|row| {
                row.enclosing == enclosing
                    && row.target == PersistenceTarget::CharacterDatabase
                    && row.operation == PersistenceOperation::PoolAccess
            }),
            "missing pool-access row for {enclosing}"
        );
    }
}

#[test]
fn persistence_inventory_inventories_registered_macros_across_source_files() {
    let definitions = ClassifiedPersistenceSource {
        classification: "database_adapter_core",
        package: "fixture",
        module: "crate::macros",
        source_path: "src/macros.rs",
        inherited_cfg: &[],
        source: r#"
                macro_rules! hidden_query { () => { sqlx::query("SELECT 1") } }
            "#,
    };
    let consumer = ClassifiedPersistenceSource {
        classification: "database_adapter_core",
        package: "fixture",
        module: "crate::consumer",
        source_path: "src/consumer.rs",
        inherited_cfg: &[],
        source: r#"
                fn persistent() { consume(hidden_query!()); }
                fn clean() { consume(1_u8); }
            "#,
    };

    let baseline = inventory_persistence_accesses(&[consumer, definitions]).unwrap();
    assert!(baseline.accesses.iter().any(|row| {
        row.enclosing == "fn persistent"
            && row.target == PersistenceTarget::Sqlx
            && row.operation == PersistenceOperation::MacroReference
            && row.symbol == "hidden_query"
    }));
    assert!(
        !baseline
            .accesses
            .iter()
            .any(|row| row.enclosing == "fn clean")
    );
}

#[test]
fn persistence_inventory_tracks_module_values_across_source_files() {
    let values = ClassifiedPersistenceSource {
        classification: "database_adapter_core",
        package: "fixture",
        module: "crate::values",
        source_path: "src/values.rs",
        inherited_cfg: &[],
        source: r#"
                static POOL: std::sync::OnceLock<sqlx::MySqlPool> = std::sync::OnceLock::new();
                const DATABASE: wow_database::CharacterDatabase = unreachable!();
                static CLEAN: std::sync::OnceLock<u8> = std::sync::OnceLock::new();
            "#,
    };
    let consumer = ClassifiedPersistenceSource {
        classification: "database_adapter_core",
        package: "fixture",
        module: "crate::consumer",
        source_path: "src/consumer.rs",
        inherited_cfg: &[],
        source: r#"
                fn persistent_pool() { consume(crate::values::POOL.get().unwrap().acquire()); }
                fn persistent_database() { consume(crate::values::DATABASE.pool()); }
                fn clean() { consume(crate::values::CLEAN.get()); }
            "#,
    };

    let baseline = inventory_persistence_accesses(&[consumer, values]).unwrap();
    assert!(
        baseline.accesses.iter().any(|row| {
            row.enclosing == "fn persistent_pool"
                && row.target == PersistenceTarget::MySqlPool
                && row.operation == PersistenceOperation::PoolAccess
        }),
        "{:#?}",
        baseline.accesses
    );
    assert!(baseline.accesses.iter().any(|row| {
        row.enclosing == "fn persistent_database"
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
fn persistence_inventory_tracks_inline_module_values_across_source_files() {
    let values = ClassifiedPersistenceSource {
        classification: "database_adapter_core",
        package: "fixture",
        module: "crate::values",
        source_path: "src/values.rs",
        inherited_cfg: &[],
        source: r#"
                mod nested {
                    static POOL: std::sync::OnceLock<sqlx::MySqlPool> = std::sync::OnceLock::new();
                }
            "#,
    };
    let consumer = ClassifiedPersistenceSource {
        classification: "database_adapter_core",
        package: "fixture",
        module: "crate::consumer",
        source_path: "src/consumer.rs",
        inherited_cfg: &[],
        source: r#"
                fn persistent() {
                    consume(crate::values::nested::POOL.get().unwrap().acquire());
                }
            "#,
    };
    let baseline = inventory_persistence_accesses(&[consumer, values]).unwrap();
    assert!(baseline.accesses.iter().any(|row| {
        row.enclosing == "fn persistent"
            && row.target == PersistenceTarget::MySqlPool
            && row.operation == PersistenceOperation::PoolAccess
    }));
}

#[test]
fn persistence_inventory_applies_mutable_helper_writes_across_source_files() {
    let helper = ClassifiedPersistenceSource {
        classification: "database_adapter_core",
        package: "fixture",
        module: "crate::helper",
        source_path: "src/helper.rs",
        inherited_cfg: &[],
        source: r#"
                pub fn install(slot: &mut Option<wow_database::CharacterDatabase>) {
                    *slot = Some(unreachable!());
                }
            "#,
    };
    let consumer = ClassifiedPersistenceSource {
        classification: "database_adapter_core",
        package: "fixture",
        module: "crate::consumer",
        source_path: "src/consumer.rs",
        inherited_cfg: &[],
        source: r#"
                fn persistent() {
                    let mut slot = None;
                    crate::helper::install(&mut slot);
                    consume(slot.unwrap().pool());
                }
            "#,
    };
    let baseline = inventory_persistence_accesses(&[consumer, helper]).unwrap();
    assert!(baseline.accesses.iter().any(|row| {
        row.enclosing == "fn persistent" && row.operation == PersistenceOperation::PoolAccess
    }));
}

#[test]
fn persistence_inventory_applies_mutable_method_effects_across_source_files() {
    let helper = ClassifiedPersistenceSource {
        classification: "database_adapter_core",
        package: "fixture",
        module: "crate::helper",
        source_path: "src/helper.rs",
        inherited_cfg: &[],
        source: r#"
                pub struct Holder { pub slot: Option<wow_database::CharacterDatabase> }
                impl Holder {
                    pub fn install(&mut self) { self.slot = Some(unreachable!()); }
                    pub fn fill(&self, slot: &mut Option<wow_database::CharacterDatabase>) {
                        *slot = Some(unreachable!());
                    }
                }
            "#,
    };
    let consumer = ClassifiedPersistenceSource {
        classification: "database_adapter_core",
        package: "fixture",
        module: "crate::consumer",
        source_path: "src/consumer.rs",
        inherited_cfg: &[],
        source: r#"
                fn persistent(mut holder: crate::helper::Holder) {
                    let mut other = None;
                    holder.install();
                    holder.fill(&mut other);
                    consume(holder.slot.unwrap().pool());
                    consume(other.unwrap().pool());
                }
            "#,
    };
    let baseline = inventory_persistence_accesses(&[consumer, helper]).unwrap();
    assert_eq!(
        baseline
            .accesses
            .iter()
            .filter(|row| row.enclosing == "fn persistent"
                && row.operation == PersistenceOperation::PoolAccess)
            .count(),
        2
    );
}
