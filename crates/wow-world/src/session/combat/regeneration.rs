// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Canonical Player power and health regeneration tick: the body moved to
//! `wow_world_entities::combat_session` (#1363 slice 7a); the test entry points
//! stay here.

#[cfg(test)]
#[path = "../../../unit_tests/session/combat/regeneration/f3_shims.rs"]
mod f3_shims;
