// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Account-data and client-state packet handlers.

mod account_data;
mod client_state;

#[cfg(test)]
#[path = "account_data/tests/mod.rs"]
mod tests;
