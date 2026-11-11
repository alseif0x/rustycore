//! SQL-free projections of Player quest save queues and periodic change state.
use std::collections::{BTreeMap, BTreeSet};

/// Save intent is preserved separately from final quest membership. No Default:
/// construction must supply the cleaning policy, queues and periodic changes.
pub struct NewQuestSave {
    pub keep_abandoned: bool,
    pub statuses: BTreeMap<u32, NewQuestChange>,
    pub rewarded: BTreeMap<u32, NewRewardedQuestChange>,
    /// None means unchanged; Some retains the source changed-state semantics.
    pub daily: Option<NewDailyQuestSave>,
    pub weekly: Option<BTreeSet<u32>>,
    pub seasonal: Option<Vec<NewSeasonalQuestGroup>>,
    pub monthly: Option<BTreeSet<u32>>,
}

pub struct NewDailyQuestSave {
    /// Preserve the source int32 list order and cast each ID to SQL uint32.
    pub quests: Vec<i32>,
    pub dungeon_quests: BTreeSet<u32>,
    pub last_time: i64,
}

pub struct NewSeasonalQuestGroup {
    pub event: u32,
    /// Preserve the observed iteration order of both source unordered maps.
    pub quests: Vec<NewSeasonalQuest>,
}

pub struct NewSeasonalQuest {
    pub quest: u32,
    pub completed_time: i64,
}

pub enum NewQuestChange {
    /// None represents a queued default save whose canonical status is absent.
    Default(Option<NewQuestStatus>),
    Delete,
}

pub struct NewQuestStatus {
    pub status: u8,
    pub explored: bool,
    pub accept_time: i64,
    pub end_time: i64,
    /// Already projected in template objective iteration order.
    pub objectives: Vec<NewQuestObjective>,
    /// Preserve the observed source iteration order, including zero IDs.
    pub spawn_tracking: Vec<NewQuestSpawnTracking>,
}

pub struct NewQuestObjective {
    /// Source int8 storage indices are represented as SQL uint8 values.
    pub index: u8,
    pub count: i32,
}

pub struct NewQuestSpawnTracking {
    pub index: u8,
    pub id: u32,
}

pub enum NewRewardedQuestChange {
    Default,
    Delete,
    ForceDelete,
}
