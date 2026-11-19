// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Group invariant tests for [`super`].
//!
//! Moved verbatim from `wow_network::group_registry` by issue #137. Following
//! the extraction convention of issue #214, the only textual change is
//! dedenting by one level, which lets rustfmt collapse some argument lists and
//! drop their trailing commas.

#![cfg(test)]

#[path = "tests/instances.rs"]
mod instances;

#[path = "tests/leader_looter.rs"]
mod leader_looter;

#[path = "tests/load_db.rs"]
mod load_db;

#[path = "tests/membership.rs"]
mod membership;

#[path = "tests/ready_check.rs"]
mod ready_check;

#[path = "tests/target_icons.rs"]
mod target_icons;
