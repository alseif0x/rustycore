// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use std::path::{Path, PathBuf};
use std::process::ExitCode;

fn usage() -> ExitCode {
    eprintln!(
        "usage: session-ownership-check check [--policy PATH]\n       \
         session-ownership-check check --syntax-only\n       \
         session-ownership-check print-baseline\n       \
         session-ownership-check print-persistence-baseline\n       \
         session-ownership-check print-persistence-policy [--from-snapshot PATH]"
    );
    ExitCode::FAILURE
}

/// Stack for the scan thread.
///
/// The scan recurses over every item of every module in the workspace, and
/// `session.rs` alone is a six-figure line count, so the depth is a property of
/// the repository rather than of any one input. `RUST_MIN_STACK` cannot supply
/// it: that variable sizes stacks for threads Rust spawns, and this analyzer
/// spawned none -- it called the scan straight from `main`, whose stack the
/// process inherits from the OS. Setting it in CI documented a budget that was
/// never applied, which is consistent with the scan still dying on a stack the
/// variable never touched.
const SCAN_STACK_BYTES: usize = 1 << 30;

fn main() -> ExitCode {
    // Sized here, on a thread that actually gets the size.
    match std::thread::Builder::new()
        .name("session-ownership-scan".to_owned())
        .stack_size(SCAN_STACK_BYTES)
        .spawn(run)
    {
        Ok(handle) => match handle.join() {
            Ok(code) => code,
            Err(_) => {
                eprintln!("session ownership check panicked");
                ExitCode::FAILURE
            }
        },
        Err(error) => {
            eprintln!("cannot start the session ownership scan thread: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> ExitCode {
    let arguments: Vec<_> = std::env::args_os().skip(1).collect();
    let result = match arguments.as_slice() {
        [command] if command == "check" => {
            handler_contract_check::check_session_ownership_repository(None)
        }
        [command, flag] if command == "check" && flag == "--syntax-only" => {
            handler_contract_check::check_session_ownership_repository_syntax_only(None)
        }
        [command, flag, policy] if command == "check" && flag == "--policy" => {
            let policy = PathBuf::from(policy);
            handler_contract_check::check_session_ownership_repository(Some(&policy))
        }
        [command] if command == "print-baseline" => {
            handler_contract_check::print_session_ownership_baseline()
        }
        [command] if command == "print-persistence-baseline" => {
            handler_contract_check::print_persistence_access_baseline()
        }
        [command] if command == "print-persistence-policy" => {
            handler_contract_check::print_persistence_boundary_policy()
        }
        [command, flag, snapshot]
            if command == "print-persistence-policy" && flag == "--from-snapshot" =>
        {
            handler_contract_check::print_persistence_boundary_policy_from_snapshot(Path::new(
                snapshot,
            ))
        }
        _ => return usage(),
    };
    match result {
        Ok(report) => {
            println!("{report}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("session ownership check failed: {error}");
            ExitCode::FAILURE
        }
    }
}
