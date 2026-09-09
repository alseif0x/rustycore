// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! SkillLineAbility.db2 + SkillRaceClassInfo.db2 reader.
//!
//! Determines which spells each race/class/level should auto-learn,
//! replicating TrinityCore C++ `LearnDefaultSkills()` → `SetSkill()` →
//! `LearnSkillRewardedSpells()`.

mod loaded;

use std::collections::{BTreeMap, HashMap};
use std::path::Path;

use anyhow::{Context, Result};
use tracing::info;

use crate::Db2HotfixRemovalStoreLikeCpp;
use crate::entities_movement::CreatureFamilyEntry;
use crate::skill_talent::{SkillLineAcquisitionPayloadLikeCpp, SkillLineStore};
use crate::wdc4::Wdc4Reader;

// ── Records ─────────────────────────────────────────────────────────

mod ops_1;
mod state_1;
mod state_2;
#[allow(unused_imports)]
pub use ops_1::*;
#[allow(unused_imports)]
pub use state_1::*;
#[allow(unused_imports)]
pub use state_2::*;

#[cfg(test)]
#[path = "skill/tests/mod.rs"]
mod tests;

// ── Store ───────────────────────────────────────────────────────────

// ── Tests ────────────────────────────────────────────────────────────
