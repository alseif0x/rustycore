// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Loot permission and master-loot authority: the bodies moved to
//! `wow_world_loot_ops` (#1363 slice 5a); the unit-test entry points stay.

#[cfg(test)]
use super::*;

#[cfg(test)]
#[path = "../../../unit_tests/handlers/loot/authority/test_only.rs"]
mod test_only;
