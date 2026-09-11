//! Data-module allowance stays separate from actual registration capability.

use super::*;
use serde_json::json;

#[test]
fn data_module_glob_requires_a_local_declaration_and_incapable_package() {
    let source = "pub mod inventory; pub use inventory::*;";
    assert!(data_module_alias_violations(source).unwrap().is_empty());
    assert!(!registration_alias_violations(source).unwrap().is_empty());
    for source in [
        "pub use inventory::*;",
        "pub mod inventory; pub use ::inventory::*;",
        "pub mod inventory; pub use inventory::submit;",
        "pub mod inventory; pub use inventory::submit as hidden;",
        "pub mod inventory; pub use inv as inventory;",
        "pub mod inventory; mod child { pub use inventory::*; }",
        "pub mod inventory; extern crate inv as inventory;",
    ] {
        assert!(
            !data_module_alias_violations(source).unwrap().is_empty(),
            "{source}"
        );
    }
}

#[test]
fn local_data_inventory_does_not_exempt_handler_macro_aliases() {
    let source = "mod inventory; pub use inventory::*; use other::register_move;";
    let errors = data_module_alias_violations(source).unwrap();
    assert!(
        errors
            .iter()
            .any(|error| error.contains("audited handler registration macro"))
    );
}

fn metadata(kind: serde_json::Value) -> serde_json::Value {
    json!({
        "packages": [
            {"id": "inv-id", "name": "inventory"},
            {"id": "facade", "name": "facade"},
            {"id": "consumer", "name": "consumer"},
            {"id": "data", "name": "data"}
        ],
        "resolve": {"nodes": [
            {"id": "inv-id", "deps": []},
            {"id": "facade", "deps": [
                {"name": "renamed", "pkg": "inv-id", "dep_kinds": [{"kind": kind, "target": "cfg(windows)"}]}
            ]},
            {"id": "consumer", "deps": [
                {"name": "facade", "pkg": "facade", "dep_kinds": [{"kind": null}]}
            ]},
            {"id": "data", "deps": []}
        ]}
    })
}

#[test]
fn renamed_target_specific_and_transitive_inventory_dependencies_retain_strict_guard() {
    assert_eq!(
        inventory_dependency_packages(&metadata(json!(null))).unwrap(),
        BTreeSet::from([
            "inv-id".to_owned(),
            "facade".to_owned(),
            "consumer".to_owned()
        ]),
    );
}

#[test]
fn dev_and_build_dependencies_do_not_create_production_inventory_capability() {
    for kind in ["dev", "build"] {
        assert_eq!(
            inventory_dependency_packages(&metadata(json!(kind))).unwrap(),
            BTreeSet::from(["inv-id".to_owned()]),
        );
    }
}

#[test]
fn missing_or_invalid_metadata_cannot_open_the_data_module_allowance() {
    inventory_dependency_packages(&json!({})).unwrap_err();
    inventory_dependency_packages(&metadata(json!("unknown"))).unwrap_err();
    let mut missing = metadata(json!(null));
    missing["resolve"]["nodes"][1]["deps"][0]["pkg"] = json!("unresolved");
    inventory_dependency_packages(&missing).unwrap_err();
    let mut incomplete = metadata(json!(null));
    incomplete["resolve"]["nodes"][1]["deps"][0]["dep_kinds"] = json!([]);
    inventory_dependency_packages(&incomplete).unwrap_err();
}
