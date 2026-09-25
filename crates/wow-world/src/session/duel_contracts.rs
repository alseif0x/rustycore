// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Duel contracts: private Session responsibility.
//! Relocated under #1233; canonical state, phase order and public paths are unchanged.

use super::ObjectGuid;

#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RepresentedCanDuelSpellCastLikeCpp {
    pub target_guid: ObjectGuid,
    pub spell_id: u32,
    pub to_the_death: bool,
}

#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RepresentedDuelRequestedLikeCpp {
    pub target_guid: ObjectGuid,
    pub arbiter_guid: ObjectGuid,
    pub gameobject_entry: u32,
    pub to_the_death: bool,
}

#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RepresentedDuelAcceptedLikeCpp {
    pub opponent_guid: ObjectGuid,
    pub arbiter_guid: ObjectGuid,
    pub countdown_ms: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg(test)]
pub(crate) enum RepresentedDuelCancelOutcomeLikeCpp {
    Interrupted,
    Surrendered,
}

#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RepresentedDuelCancelledLikeCpp {
    pub opponent_guid: ObjectGuid,
    pub outcome: RepresentedDuelCancelOutcomeLikeCpp,
    pub beg_spell_id: Option<u32>,
}
