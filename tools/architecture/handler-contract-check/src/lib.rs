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
//! enforcing `crate::handlers` as the sole registration location for the
//! audited direct and macro-generated grammar.

mod bridge_access;
mod dispatcher;
mod ownership;
mod persistence_access;
mod registrations;
mod registry_access;
mod session_ownership;
mod snapshot;

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use dispatcher::{
    DISPATCH_ARM_WITHOUT_REGISTRATION, REGISTERED_WITHOUT_DISPATCH_ARM, compare_dispatch_sides,
    dispatcher_contract_from_source,
};
use ownership::audit_registration_ownership;
use registrations::{EXPECTED_REGISTRATION_MACROS, analyze_handler_source};
use snapshot::parse_snapshot_contract;

pub use session_ownership::{
    check_repository as check_session_ownership_repository,
    print_repository_baseline as print_session_ownership_baseline,
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
    let session_path = repository_root.join("crates/wow-world/src/session.rs");
    let wow_world_crate = repository_root.join("crates/wow-world");
    let crate_root = wow_world_crate.join("src/lib.rs");

    let snapshot_source = fs::read_to_string(&snapshot_path)
        .map_err(|error| format!("cannot read {}: {error}", snapshot_path.display()))?;
    let snapshot = parse_snapshot_contract(&snapshot_source)
        .map_err(|error| format!("invalid handler contract snapshot: {error}"))?;
    let session_source = fs::read_to_string(&session_path)
        .map_err(|error| format!("cannot read {}: {error}", session_path.display()))?;
    let dispatcher = dispatcher_contract_from_source(&session_source)
        .map_err(|error| format!("invalid world-session dispatcher: {error}"))?;
    compare_dispatch_sides(
        &snapshot.opcode_names,
        &dispatcher.opcode_names,
        REGISTERED_WITHOUT_DISPATCH_ARM,
        DISPATCH_ARM_WITHOUT_REGISTRATION,
    )
    .map_err(|error| format!("world handler dispatch/registration drift:\n{error}"))?;

    let ownership = audit_registration_ownership(&repository_root)
        .map_err(|error| format!("invalid handler registration ownership:\n{error}"))?;
    let source_report = analyze_handler_source(&crate_root)
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
