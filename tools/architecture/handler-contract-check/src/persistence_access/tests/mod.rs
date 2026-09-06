//! Persistence-inventory regressions, separated by the contract under test.

use super::*;

fn inventory(source: &str) -> Result<PersistenceAccessBaseline, String> {
    inventory_for_package("fixture", source)
}

fn inventory_for_package(package: &str, source: &str) -> Result<PersistenceAccessBaseline, String> {
    inventory_persistence_accesses(&[ClassifiedPersistenceSource {
        classification: "database_adapter_core",
        package,
        module: "crate",
        source_path: "src/fixture.rs",
        inherited_cfg: &[],
        source,
    }])
}

fn operations(
    baseline: &PersistenceAccessBaseline,
) -> BTreeSet<(PersistenceTarget, PersistenceOperation, String)> {
    baseline
        .accesses
        .iter()
        .map(|record| (record.target, record.operation, record.symbol.clone()))
        .collect()
}

mod aliases_and_paths;
mod baseline_and_cfg;
mod callable_resolution;
mod conservative_flow;
mod control_flow;
mod cross_source_contracts;
mod deferred_effects;
mod dependency_exports;
mod generic_inference;
mod macro_resolution;
mod query_execution;
mod sql_classification;
mod sql_strings;
mod trait_contracts;
mod tuple_projection;
mod type_and_value_flow;
