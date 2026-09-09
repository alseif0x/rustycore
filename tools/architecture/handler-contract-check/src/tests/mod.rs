//! Handler-contract regressions.
//!
//! Separated from the tests.rs root under #660.

// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use super::check_repository;
use crate::dispatcher::{
    DispatcherContract, assert_single_dispatch_mechanism, dispatcher_contract_from_mounts,
    dispatcher_contract_from_source,
};
use crate::module_policy::{CapabilityOwner, parse_handler_module_policy};
use crate::ownership::{
    WorkspaceSourceMount, audit_package_registration_sources,
    audit_package_registration_sources_with_owner, audit_package_source_graph,
    audit_package_source_mounts, read_spliced_source, registry_capable_package_ids,
    workspace_dependency_aliases_from_metadata,
};
use crate::registrations::{
    RegistrationSourceReport, analyze_handler_mounts, analyze_inline_source, exported_macro_names,
    handler_capable_macro_definitions, handler_capable_macro_invocations, include_macro_bodies,
    inventory_registration_macro_fingerprints, registration_alias_violations,
    reject_registration_syntax_outside_handlers,
};
use crate::snapshot::{SnapshotContract, parse_snapshot_contract};
use serde_json::json;

fn dispatcher_owner() -> CapabilityOwner {
    CapabilityOwner {
        capability: "packet_dispatcher".to_owned(),
        package: "wow-world".to_owned(),
        module: "crate::session".to_owned(),
        allow_descendants: true,
        tracking_issue: 152,
    }
}

fn dispatcher_body(opcode: &str) -> String {
    format!(
        r#"
            impl crate::session::WorldSession {{
                async fn dispatch_packet(&mut self) {{
                    match opcode {{
                        ClientOpcodes::{opcode} => {{}},
                        _ => {{}},
                    }}
                }}
            }}
        "#
    )
}

fn fixture_workspace_mounts(
    package: &str,
    package_root: &Path,
    crate_root: &Path,
) -> Vec<WorkspaceSourceMount> {
    let (mounts, _) = audit_package_source_mounts(package_root, &[crate_root.to_owned()])
        .expect("fixture module graph resolves");
    mounts
        .into_iter()
        .map(|(source_path, contexts)| WorkspaceSourceMount {
            package: package.to_owned(),
            source: fs::read_to_string(&source_path).expect("read fixture source"),
            source_path,
            contexts,
        })
        .collect()
}

fn source_graph_fixture(name: &str) -> PathBuf {
    let unique = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock after epoch")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "handler-contract-check-{name}-{}-{unique}",
        std::process::id()
    ))
}

mod scenarios_1;
mod scenarios_2;
mod scenarios_3;
