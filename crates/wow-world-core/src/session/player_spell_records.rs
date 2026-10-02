// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RepresentedPlayerSkillStateLikeCpp {
    Unchanged,
    Changed,
    New,
    Deleted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RepresentedPlayerSkillLikeCpp {
    pub skill_id: u16,
    pub step: u16,
    pub value: u16,
    pub max: u16,
    pub profession_slot: i8,
    pub state: RepresentedPlayerSkillStateLikeCpp,
}
