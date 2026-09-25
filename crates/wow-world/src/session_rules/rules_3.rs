// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Receiver-free Session rules, part 3.
//!
//! Aura projections and melee-damage rules are grouped into focused child
//! modules and remain re-exported through the session-rules facade.

#[path = "rules_3/aura_effects.rs"]
pub(super) mod aura_effects;
#[path = "rules_3/melee_damage.rs"]
pub(super) mod melee_damage;
