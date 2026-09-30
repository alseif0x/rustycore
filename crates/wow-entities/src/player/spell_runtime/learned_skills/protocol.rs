//! Scalar facts for the existing gain and downgrade paths, without catalogs.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LearnedSkillNode {
    pub skill_id: u16,
    pub step: u16,
    pub value: u16,
    pub max_value: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LearnedSkillLookup {
    Present(LearnedSkillNode),
    Absent,
    Unavailable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LearnedSkillRange {
    Unavailable,
    Language { always_max: bool },
    Level { always_max: bool },
    Mono { always_max: bool },
    Rank { always_max: bool, tier_id: i16 },
    None { always_max: bool },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LearnedSkillWrite {
    pub skill_id: u16,
    pub step: u16,
    pub value: u16,
    pub max_value: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LearnedSkillStep {
    GainNode(u32),
    PreviousRank(u32),
    PreviousNode(u32),
    FirstRank(u32),
    Value(u16),
    Maximum(u16),
    Range(u16),
    LevelMaximum,
    TierMaximum { tier_id: i16, index: u32 },
    Write(LearnedSkillWrite),
    Done(bool),
}

pub enum LearnedSkillInput {
    GainNode(LearnedSkillLookup),
    PreviousNode(Option<LearnedSkillNode>),
    Rank(u32),
    Value(Option<u16>),
    Maximum(Option<u16>),
    Range(LearnedSkillRange),
    LevelMaximum(u16),
    TierMaximum(Option<u32>),
    Applied,
}
