// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Effect learning: re-exported from `wow-world-spell` (#1263 F5 move,
//! #1363 A5).

pub(crate) use wow_world_spell::{EffectLearningRuntimeLikeCpp, execute_effect_learning_like_cpp};
#[cfg(test)]
pub(crate) use wow_world_spell::{
    apply_base_learning_like_cpp, may_shallow_fallback_after_profession_plan_error_like_cpp,
};
