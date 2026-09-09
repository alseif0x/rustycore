// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Creature AI — state machine for NPC/mob behavior.
//!
//! Implements idle wandering, random movement, aggro detection, and
//! basic melee combat for server-controlled creatures.

use std::collections::VecDeque;
use std::time::Instant;

use rand::{Rng, SeedableRng, rngs::StdRng};
use wow_core::{ObjectGuid, Position, random_resize_vec_like_cpp};
use wow_instances::BossAiRef;

// ── CreatureAISelector ────────────────────────────────────────────

mod ai;
#[cfg(test)]
#[path = "ai/tests/mod.rs"]
mod tests;
pub use ai::*;

// ── CreatureAI::CanAIAttack ───────────────────────────────────────

// ── UnitAI::SelectTarget ──────────────────────────────────────────

// ── CreatureAI::EnterEvadeMode ────────────────────────────────────

// ── CreatureAI::TriggerAlert ──────────────────────────────────────

// ── CreatureAI::DoZoneInCombat ────────────────────────────────────

// ── ScriptedAI::SummonList ────────────────────────────────────────

// ── CreatureState ──────────────────────────────────────────────────

// ── CreatureAI ────────────────────────────────────────────────────

// ── Position distance helper ──────────────────────────────────────
// Position already has .distance() from wow-core; we define this
// convenience method here for internal use.
