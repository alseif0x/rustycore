// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Read-only stat projections and level-up deltas.

use super::*;

#[cfg(test)]
#[path = "../../../unit_tests/handlers/character/stats_queries/test_only.rs"]
mod test_only;
