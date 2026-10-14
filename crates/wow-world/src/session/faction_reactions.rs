// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Faction reactions: private Session responsibility.
//! Relocated under #1233; canonical state, phase order and public paths are unchanged.

pub(crate) use wow_world_core::session::{
    RepresentedFactionReactionInputLikeCpp, ReputationGainSourceLikeCpp,
};

#[cfg(test)]
#[path = "../../unit_tests/session/faction_reactions/f3_shims.rs"]
mod f3_shims;
