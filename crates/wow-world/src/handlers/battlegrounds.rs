// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Battleground and arena packet handlers.
//!
//! The arena-team family moved to `wow-world-social` in #1263 F5; the
//! battleground family stays here until its turn.

mod battleground_host;
mod pvp;

#[cfg(test)]
mod test_shims;

#[cfg(test)]
#[path = "../../unit_tests/handlers/battlegrounds/tests/mod.rs"]
mod tests;
