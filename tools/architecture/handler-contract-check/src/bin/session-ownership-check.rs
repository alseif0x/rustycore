// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use std::path::PathBuf;
use std::process::ExitCode;

fn usage() -> ExitCode {
    eprintln!(
        "usage: session-ownership-check check [--policy PATH]\n       \
         session-ownership-check print-baseline"
    );
    ExitCode::FAILURE
}

fn main() -> ExitCode {
    let arguments: Vec<_> = std::env::args_os().skip(1).collect();
    let result = match arguments.as_slice() {
        [command] if command == "check" => {
            handler_contract_check::check_session_ownership_repository(None)
        }
        [command, flag, policy] if command == "check" && flag == "--policy" => {
            let policy = PathBuf::from(policy);
            handler_contract_check::check_session_ownership_repository(Some(&policy))
        }
        [command] if command == "print-baseline" => {
            handler_contract_check::print_session_ownership_baseline()
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
