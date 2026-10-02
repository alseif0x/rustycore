// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Faction reactions: private Session responsibility.
//! Relocated under #1233; canonical state, phase order and public paths are unchanged.

use super::WorldSession;

pub(crate) use wow_world_core::session::{
    RepresentedFactionReactionInputLikeCpp, RepresentedGetReactionInputLikeCpp,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::session) struct AttackReputationFactionSnapshotLikeCpp {
    pub(in crate::session) faction_id: u32,
    pub(in crate::session) contested_guard: bool,
    pub(in crate::session) can_have_reputation: Option<bool>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ReputationGainSourceLikeCpp {
    Kill,
    Quest,
    DailyQuest,
    WeeklyQuest,
    MonthlyQuest,
    RepeatableQuest,
    Spell,
}

impl WorldSession {
    pub(crate) const fn reset_schedule_like_cpp(&self) -> wow_instances::ResetSchedule {
        self.config.reset_schedule_like_cpp()
    }
}

#[cfg(test)]
#[path = "../../unit_tests/session/faction_reactions/f3_shims.rs"]
mod f3_shims;
