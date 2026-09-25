// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Travel and worldport packet handlers.

mod travel;
mod worldport_create;

#[cfg(test)]
#[path = "travel/tests/mod.rs"]
mod tests;
