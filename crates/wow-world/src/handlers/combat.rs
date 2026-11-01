// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Combat packet handlers.
//!
//! Handles CMSG_ATTACK_SWING, CMSG_ATTACK_STOP, CMSG_SET_SHEATHED.
//!
//! Reference: C++ `WorldSession::HandleAttack*Opcode`
//! (`src/server/game/Handlers/CombatHandler.cpp`).
//!
//! Registration moved to the `wow-world-application` `ApplicationCombat` area
//! registrar under #1263 F5 `#1263 F5 remaining families`; the bodies moved to
//! `wow_world_entities::player_attack` under #1363 slice 4a.

// ── Handler implementations ───────────────────────────────────────

#[cfg(test)]
mod test_shims;
