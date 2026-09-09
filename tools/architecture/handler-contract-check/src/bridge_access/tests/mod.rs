//! Bridge access scan regression scenarios.
//!
//! Separated from the bridge_access.rs root under #660.

use super::*;
use std::fs;
use std::path::Path;

fn source<'a>(text: &'a str) -> BridgeSource<'a> {
    BridgeSource {
        package: "fixture",
        module: "crate::fixture",
        source_path: "src/fixture.rs",
        inherited_cfg: &[],
        source: text,
    }
}

fn inventory(text: &str) -> Result<BridgeAccessBaseline, String> {
    inventory_bridge_accesses(&[source(text)])
}

mod scenarios_1;
