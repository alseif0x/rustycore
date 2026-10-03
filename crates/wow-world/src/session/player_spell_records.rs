// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Player skill records remain owned by wow-world-core.

pub(crate) use wow_world_core::session::{
    RepresentedPlayerSkillLikeCpp, RepresentedPlayerSkillStateLikeCpp,
};

#[cfg(test)]
pub(crate) use wow_world_core::session::is_non_durable_skill_tombstone_like_cpp;
pub(in crate::session) use wow_world_core::session::{
    canonical_player_skill_record_like_cpp, represented_player_skill_record_like_cpp,
};
