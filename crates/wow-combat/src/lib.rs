// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Deterministic combat rules over caller-resolved values.
//!
//! The first boundary is the existing represented melee attack table, moved
//! from `wow-world` under #1233 without expanding its gameplay coverage.
//! Callers own canonical state, randomness, clocks, catalogs and publication;
//! this crate owns only the arithmetic and has no dependencies.

mod attack_table;
mod damage;
mod facts;

pub use attack_table::{
    MELEE_OUTCOME_ROLL_MAX_LIKE_CPP, RepresentedMeleeOutcomeInputsLikeCpp,
    RepresentedMeleeOutcomeLikeCpp, melee_outcome_like_cpp,
};
pub use damage::{
    CREATURE_BLOCK_PERCENT_LIKE_CPP, melee_outcome_damage_like_cpp, player_block_percent_like_cpp,
};
pub use facts::{
    RepresentedMeleeAttackerFactsLikeCpp, RepresentedMeleeVictimFactsLikeCpp,
    melee_outcome_inputs_like_cpp,
};
