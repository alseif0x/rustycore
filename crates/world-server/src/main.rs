// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Thin process entry point for the world server.

use std::process::ExitCode;

use anyhow::Result;

#[tokio::main]
async fn main() -> Result<ExitCode> {
    world_server::run(std::env::args().skip(1).collect()).await
}
