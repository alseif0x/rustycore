// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use std::process::ExitCode;

fn main() -> ExitCode {
    let mut arguments = std::env::args_os();
    let _binary = arguments.next();
    match (arguments.next(), arguments.next()) {
        (Some(command), None) if command == "check" => {
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
        _ => {
            eprintln!("usage: handler-contract-check check");
            ExitCode::FAILURE
        }
    }
}
