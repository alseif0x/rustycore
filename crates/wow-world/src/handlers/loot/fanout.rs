// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Loot packet fanout: the bodies moved to `wow_world_loot_ops` (#1363 slice
//! 5b); the unit-test entry points stay.

#[cfg(test)]
use super::*;

#[cfg(test)]
#[path = "../../../unit_tests/handlers/loot/fanout/f3_shims.rs"]
mod f3_shims;
