#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RepresentedPendingBind {
    pub map_id: u32,
    pub instance_id: u32,
    pub completed_mask: u32,
    pub time_until_lock_ms: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RepresentedAdventureMapStartQuestLikeCpp {
    pub quest_id: u32,
    pub adventure_map_poi_id: u32,
    pub player_condition_id: u32,
}

#[cfg(any(test, feature = "test-fixtures"))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RepresentedAreaZoneCriteriaLikeCpp {
    EnterArea(u32),
    LeaveArea(u32),
    EnterTopLevelArea(u32),
    LeaveTopLevelArea(u32),
}
