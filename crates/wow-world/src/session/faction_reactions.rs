// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Faction reactions: private Session responsibility.
//! Relocated under #1233; canonical state, phase order and public paths are unchanged.

use super::WorldSession;

pub(crate) use wow_world_core::session::{
    RepresentedFactionReactionInputLikeCpp, RepresentedGetReactionInputLikeCpp,
    ReputationGainSourceLikeCpp,
};

impl WorldSession {
    pub(crate) const fn reset_schedule_like_cpp(&self) -> wow_instances::ResetSchedule {
        self.config.reset_schedule_like_cpp()
    }
}

#[cfg(test)]
#[path = "../../unit_tests/session/faction_reactions/f3_shims.rs"]
mod f3_shims;
