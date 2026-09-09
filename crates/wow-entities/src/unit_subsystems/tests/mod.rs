// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Unit subsystem tests for [`super`].
//!
//! Moved from the inline `unit_subsystems_tests` module by issue #226.

#![cfg(test)]

use super::*;

#[path = "unit_subsystems_tests/mod.rs"]
mod unit_subsystems_tests;
