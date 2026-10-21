// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use std::process::ExitCode;

fn main() -> ExitCode {
    let (root, arguments) =
        match handler_contract_check::take_root_argument(std::env::args_os().skip(1).collect()) {
            Ok(split) => split,
            Err(error) => {
                eprintln!("{error}");
                return usage();
            }
        };
    match arguments.as_slice() {
        [command] if command == "check" => {
            if let Err(error) =
                handler_contract_check::select_scan_root("handler-contract-check", root.as_deref())
            {
                eprintln!("handler contract check failed: {error}");
                return ExitCode::FAILURE;
            }
            match handler_contract_check::check_repository() {
                Ok(report) => {
                    println!("{report}");
                    ExitCode::SUCCESS
                }
                Err(error) => {
                    eprintln!("handler contract check failed: {error}");
                    ExitCode::FAILURE
                }
            }
        }
        _ => usage(),
    }
}

fn usage() -> ExitCode {
    eprintln!("usage: handler-contract-check check [--root PATH]");
    ExitCode::FAILURE
}
