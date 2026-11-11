//! Frozen character progress projections; canonical state/flag application is absent.

pub struct NewAchievementSave {
    /// Preserve the source unordered completed-achievement map iteration order.
    pub completed: Vec<NewCompletedAchievement>,
    pub progress: Vec<NewCriteriaProgress>,
}

pub struct NewCompletedAchievement {
    pub achievement: u32,
    pub changed: bool,
    pub date: i64,
}

/// Preserve source unordered progress-map order and its resolved changed flags.
pub struct NewCriteriaProgress {
    pub criteria: u32,
    pub changed: bool,
    pub counter: u64,
    pub date: i64,
}

/// The containing map is ordered by RepListID; faction is the value's ID.
pub struct NewReputation {
    pub faction: u32,
    pub need_save: bool,
    pub standing: i32,
    pub flags: u16,
}

pub struct NewQuestCriteriaSave {
    /// Unique IDs in the observed source unordered-set iteration order.
    pub completed_objectives: Vec<u32>,
    pub progress: Vec<NewCriteriaProgress>,
}
