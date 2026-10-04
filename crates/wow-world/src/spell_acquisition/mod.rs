// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! World adapters and application of player spell/skill acquisition plans.
//!
//! `wow-spell-acquisition` owns deterministic planning, models and evidence
//! authorities. This facade keeps existing callers stable; Session snapshot
//! resolution, persistence, runtime installation and publication stay here.

use std::collections::BTreeMap;

#[cfg(test)]
pub(crate) use std::collections::BTreeSet;

mod adapter;
mod application;
mod effect_learning;
mod runtime_adapter;

pub(crate) use application::*;
pub(crate) use effect_learning::*;
pub(crate) use wow_spell_acquisition::*;
pub(crate) use wow_world_application::{
    execute_trainer_acquisition_like_cpp, TrainerAcquisitionCompletionLikeCpp,
    TrainerAcquisitionPublicationLikeCpp, TrainerAcquisitionResultLikeCpp,
    TrainerAcquisitionRuntimeLikeCpp,
};

#[cfg(test)]
#[path = "../../unit_tests/spell_acquisition/tests/mod.rs"]
mod tests;
