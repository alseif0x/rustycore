// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use std::collections::BTreeSet;
use std::path::PathBuf;

use crate::ownership::{SourceMountContext, WorkspaceSourceMount};
use crate::registrations::validate_composition_mounts;

fn mount(
    package: &str,
    logical_module_path: &str,
    source_path: &str,
    source: &str,
) -> WorkspaceSourceMount {
    WorkspaceSourceMount {
        package: package.to_owned(),
        source_path: PathBuf::from(source_path),
        contexts: BTreeSet::from([SourceMountContext {
            logical_module_path: logical_module_path.to_owned(),
            cfg: Vec::new(),
            production_possible: true,
            test_possible: true,
        }]),
        source: source.to_owned(),
    }
}

fn actual_mounts() -> Vec<WorkspaceSourceMount> {
    vec![
        mount(
            "world-server",
            "crate::handler_registry",
            "crates/world-server/src/handler_registry.rs",
            include_str!("../../../../../crates/world-server/src/handler_registry.rs"),
        ),
        mount(
            "wow-world",
            "crate::session::registry",
            "crates/wow-world/src/session/registry.rs",
            include_str!("../../../../../crates/wow-world/src/session/registry.rs"),
        ),
        mount(
            "wow-world-inventory",
            "crate",
            "crates/wow-world-inventory/src/lib.rs",
            include_str!("../../../../../crates/wow-world-inventory/src/lib.rs"),
        ),
        mount(
            "wow-world-inventory",
            "crate::handlers",
            "crates/wow-world-inventory/src/handlers/mod.rs",
            include_str!("../../../../../crates/wow-world-inventory/src/handlers/mod.rs"),
        ),
    ]
}

fn assert_rejected(mounts: &[WorkspaceSourceMount], case: &str) {
    assert!(
        validate_composition_mounts(mounts).is_err(),
        "composition grammar accepted {case}"
    );
}

#[test]
fn composition_guard_accepts_actual_production_fixture_and_both_exact_facades() {
    validate_composition_mounts(&actual_mounts())
        .expect("actual normal composer, fixture dispatch, and two facades match");
}

#[test]
fn composition_guard_rejects_missing_duplicate_nested_and_aliased_domain_calls() {
    let mut missing_legacy = actual_mounts();
    missing_legacy[0].source = missing_legacy[0]
        .source
        .replace("    register_remaining_handlers_like_cpp(&mut builder)?;\n", "");
    assert_rejected(&missing_legacy, "missing legacy composer call");

    let mut duplicate = actual_mounts();
    duplicate[0].source = duplicate[0].source.replace(
        "    register_remaining_handlers_like_cpp(&mut builder)?;",
        "    wow_world_inventory::register_inventory_handlers_like_cpp::<WorldSession, SessionHandlerCatalogsLikeCpp>(&mut builder)?;\n    register_remaining_handlers_like_cpp(&mut builder)?;",
    );
    assert_rejected(&duplicate, "duplicate domain registration");

    let call = "    wow_world_inventory::register_inventory_handlers_like_cpp::<\n        WorldSession,\n        SessionHandlerCatalogsLikeCpp,\n    >(&mut builder)?;";
    let mut nested = actual_mounts();
    nested[0].source = nested[0].source.replace(
        call,
        "    if true {\n        wow_world_inventory::register_inventory_handlers_like_cpp::<\n            WorldSession,\n            SessionHandlerCatalogsLikeCpp,\n        >(&mut builder)?;\n    }",
    );
    assert_rejected(&nested, "conditional/nested registration");

    let mut aliased = actual_mounts();
    aliased[0].source = aliased[0]
        .source
        .replace("use wow_world::session::registry::{", "use wow_world_inventory as inventory_owner;\nuse wow_world::session::registry::{")
        .replace("wow_world_inventory::register_inventory_handlers_like_cpp::<", "inventory_owner::register_inventory_handlers_like_cpp::<");
    assert_rejected(&aliased, "alternate registrar alias");
}

#[test]
fn composition_guard_rejects_wrong_mount_and_noncanonical_legacy_import() {
    let mut wrong_module = actual_mounts();
    wrong_module[0].contexts = BTreeSet::from([SourceMountContext {
        logical_module_path: "crate::handlers::other".to_owned(),
        cfg: Vec::new(),
        production_possible: true,
        test_possible: true,
    }]);
    assert_rejected(&wrong_module, "composer mounted outside its owner");

    let mut aliased_legacy = actual_mounts();
    aliased_legacy[0].source = aliased_legacy[0].source.replace(
        "    register_remaining_handlers_like_cpp,\n",
        "    register_remaining_handlers_like_cpp as register_legacy_handlers,\n",
    );
    assert_rejected(&aliased_legacy, "aliased legacy provider import");
}

#[test]
fn composition_guard_rejects_fixture_gate_changes_and_inexact_facades() {
    let mut ungated_fixture = actual_mounts();
    ungated_fixture[1].source = ungated_fixture[1].source.replace(
        "#[cfg(any(test, feature = \"test-fixtures\"))]\n#[must_use]\npub fn build_dispatch_table",
        "#[must_use]\npub fn build_dispatch_table",
    );
    assert_rejected(&ungated_fixture, "missing test-fixtures gate");

    let mut altered_expect = actual_mounts();
    altered_expect[1].source = altered_expect[1]
        .source
        .replace("invalid duplicate packet handler composition", "duplicate handler");
    assert_rejected(&altered_expect, "changed fixture expect contract");

    let mut missing_root_facade = actual_mounts();
    missing_root_facade[2].source = missing_root_facade[2]
        .source
        .replace("    register_inventory_handlers_like_cpp,\n", "");
    assert_rejected(&missing_root_facade, "missing root facade export");

    let mut aliased_handler_facade = actual_mounts();
    aliased_handler_facade[3].source = aliased_handler_facade[3].source.replace(
        "    register_inventory_handlers_like_cpp,\n",
        "    register_inventory_handlers_like_cpp as register_handlers,\n",
    );
    assert_rejected(&aliased_handler_facade, "aliased handlers facade export");

    let mut extra_handler_glob = actual_mounts();
    extra_handler_glob[3].source = extra_handler_glob[3].source.replace(
        "    register_inventory_handlers_like_cpp,\n};",
        "    register_inventory_handlers_like_cpp,\n    *,\n};",
    );
    assert_rejected(&extra_handler_glob, "extra glob in handlers facade");

    let mut extra_handler_alias = actual_mounts();
    extra_handler_alias[3].source = extra_handler_alias[3].source.replace(
        "    register_inventory_handlers_like_cpp,\n};",
        "    register_inventory_handlers_like_cpp,\n    EquipmentSetsSaveCxLikeCpp as SaveCx,\n};",
    );
    assert_rejected(&extra_handler_alias, "extra alias in handlers facade");
}
