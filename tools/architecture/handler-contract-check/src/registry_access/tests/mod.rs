//! Registry access scan regression scenarios.
//!
//! Separated from the registry_access.rs root under #660.

use super::*;

fn inventory(source: &str) -> Result<RegistryAccessBaseline, String> {
    inventory_registry_accesses(&[ProductionRegistrySource {
        package: "fixture",
        module: "crate::fixture",
        source_path: "src/fixture.rs",
        inherited_cfg: &[],
        source,
    }])
}

fn operations(
    baseline: &RegistryAccessBaseline,
) -> BTreeSet<(RegistryKind, RegistryOperation, String)> {
    baseline
        .accesses
        .iter()
        .map(|record| (record.registry, record.operation, record.symbol.clone()))
        .collect()
}

mod scenarios_1;
