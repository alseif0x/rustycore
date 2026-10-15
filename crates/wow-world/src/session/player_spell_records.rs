// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Player skill records remain owned by wow-world-core.

pub(crate) use wow_world_core::session::RepresentedPlayerSkillStateLikeCpp;

// Test-only: its production reader, the skill-record forward the continue-login
// coordinator inlined, moved to the test shims (#1363 slice 13c).
#[cfg(test)]
pub(crate) use wow_world_core::session::RepresentedPlayerSkillLikeCpp;

#[cfg(test)]
pub(crate) use wow_world_core::session::is_non_durable_skill_tombstone_like_cpp;
