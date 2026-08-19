// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Standalone architecture guard for the world-handler contract.
//!
//! This lightweight crate parses Rust source without compiling `wow-world`. It
//! ratchets the checked-in linked-registry snapshot against the active
//! `WorldSession::dispatch_packet` arms and walks the real handler module tree
//! to reject registrations hidden behind conditional compilation. Every other
//! Rust source in the `wow-world` crate is tokenized without evaluating `cfg`,
//! enforcing the logical owners declared in `handler-module-policy.json` for
//! the audited direct and macro-generated grammar.

mod bridge_access;
mod dispatcher;
mod module_policy;
mod ownership;
mod persistence_access;
mod persistence_policy;
mod registrations;
mod registry_access;
mod session_ownership;
mod snapshot;

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use dispatcher::{
    DISPATCH_ARM_WITHOUT_REGISTRATION, REGISTERED_WITHOUT_DISPATCH_ARM, compare_dispatch_sides,
    dispatcher_contract_from_mounts,
};
use module_policy::load_handler_module_policy;
use ownership::{audit_registration_ownership, workspace_source_mounts};
use registrations::{EXPECTED_REGISTRATION_MACROS, analyze_handler_mounts};
use snapshot::parse_snapshot_contract;

pub use session_ownership::{
    check_repository as check_session_ownership_repository,
    check_repository_syntax_only as check_session_ownership_repository_syntax_only,
    print_repository_baseline as print_session_ownership_baseline,
    print_repository_persistence_baseline as print_persistence_access_baseline,
    print_repository_persistence_policy as print_persistence_boundary_policy,
    print_repository_persistence_policy_from_snapshot as print_persistence_boundary_policy_from_snapshot,
};

fn repository_root() -> Result<PathBuf, String> {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .map_err(|error| format!("cannot resolve repository root: {error}"))
}

/// Validate the complete source-level world-handler contract for this checkout.
pub fn check_repository() -> Result<String, String> {
    let repository_root = repository_root()?;
    let snapshot_path = repository_root.join("tools/architecture/world-handler-contract.tsv");
    let module_policy_path = repository_root.join("tools/architecture/handler-module-policy.json");

    let snapshot_source = fs::read_to_string(&snapshot_path)
        .map_err(|error| format!("cannot read {}: {error}", snapshot_path.display()))?;
    let snapshot = parse_snapshot_contract(&snapshot_source)
        .map_err(|error| format!("invalid handler contract snapshot: {error}"))?;
    let module_policy = load_handler_module_policy(&module_policy_path)?;
    let registration_owner = module_policy.owner("handler_registration");
    let dispatcher_owner = module_policy.owner("packet_dispatcher");

    // Run the complete source/module ownership audit before the focused
    // parsers so unsupported include/macro/path shapes cannot hide source.
    let ownership = audit_registration_ownership(&repository_root, registration_owner)
        .map_err(|error| format!("invalid handler registration ownership:\n{error}"))?;
    let mounts = workspace_source_mounts(&repository_root)
        .map_err(|error| format!("invalid workspace module graph: {error}"))?;
    let dispatcher = dispatcher_contract_from_mounts(&mounts, dispatcher_owner)
        .map_err(|error| format!("invalid world-session dispatcher: {error}"))?;
    compare_dispatch_sides(
        &snapshot.opcode_names,
        &dispatcher.opcode_names,
        REGISTERED_WITHOUT_DISPATCH_ARM,
        DISPATCH_ARM_WITHOUT_REGISTRATION,
    )
    .map_err(|error| format!("world handler dispatch/registration drift:\n{error}"))?;

    let source_report = analyze_handler_mounts(&mounts, registration_owner)
        .map_err(|error| format!("invalid handler registration source contract:\n{error}"))?;
    if source_report.represented_entries() != snapshot.row_count {
        return Err(format!(
            "source registration coverage differs from the linked handler snapshot: \
             snapshot={} source={} (direct={} macro={}); audit newly introduced registration \
             syntax before changing this guard",
            snapshot.row_count,
            source_report.represented_entries(),
            source_report.direct_submissions,
            source_report.registration_macro_invocations
        ));
    }
    if source_report.direct_submissions == 0 {
        return Err("the source guard found no direct PacketHandlerEntry submissions".to_owned());
    }
    if source_report.registration_macro_names.is_empty() {
        return Err("the source guard found no PacketHandlerEntry registration macros".to_owned());
    }
    let expected_registration_macros: BTreeSet<_> = EXPECTED_REGISTRATION_MACROS
        .iter()
        .map(|name| (*name).to_owned())
        .collect();
    if source_report.registration_macro_names != expected_registration_macros {
        return Err(format!(
            "registration macro grammar changed: expected {expected_registration_macros:?}, \
             actual {:?}; audit the expansion shape before updating the guard",
            source_report.registration_macro_names
        ));
    }

    let tracked_drift =
        REGISTERED_WITHOUT_DISPATCH_ARM.len() + DISPATCH_ARM_WITHOUT_REGISTRATION.len();
    Ok(format!(
        "handler contract: PASS ({} snapshot rows; {} direct + {} macro registrations; \
         {tracked_drift} exact drift exceptions; {} production packages / {} sources clean; \
         {} workspace packages / {} production sources checked for handler-capable macro/source \
         generation surfaces; {} #[path] modules verified: {})",
        snapshot.row_count,
        source_report.direct_submissions,
        source_report.registration_macro_invocations,
        ownership.scanned_packages,
        ownership.scanned_files,
        ownership.macro_scan_packages,
        ownership.macro_scan_files,
        ownership.explicit_path_modules,
        ownership.package_names.join(", ")
    ))
}

#[cfg(test)]
mod tests;
