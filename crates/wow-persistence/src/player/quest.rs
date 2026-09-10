//! SQLx-free persistence contract for Player-owned quest state.

use crate::{PersistenceFutureLikeCpp, PersistenceOutcomeLikeCpp};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QuestObjectiveCountPersistenceLikeCpp {
    pub objective_index: u8,
    pub count: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuestStatusPersistenceLikeCpp {
    pub quest_id: u32,
    pub status: u8,
    pub explored: bool,
    pub accept_time_secs: i64,
    pub end_time_secs: i64,
    pub objectives: Vec<QuestObjectiveCountPersistenceLikeCpp>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerQuestActivePersistenceRowLikeCpp {
    pub quest_id: Option<u32>,
    pub status: Option<u8>,
    pub explored: Option<u8>,
    pub accept_time_secs: Option<i64>,
    pub end_time_secs: Option<i64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerQuestObjectivePersistenceRowLikeCpp {
    pub quest_id: Option<u32>,
    pub storage_index: Option<u8>,
    pub count: Option<i32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerQuestIdPersistenceRowLikeCpp {
    pub quest_id: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerQuestDailyPersistenceRowLikeCpp {
    pub quest_id: Option<u32>,
    pub completed_time: Option<i64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerQuestSeasonalPersistenceRowLikeCpp {
    pub quest_id: Option<u32>,
    pub event_id: Option<u32>,
    pub completed_time: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlayerQuestLoadOutcomeLikeCpp<T> {
    Loaded(Vec<T>),
    Failed { reason: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlayerQuestStatusPersistenceRequestLikeCpp {
    Save {
        owner_guid: u64,
        status: QuestStatusPersistenceLikeCpp,
    },
    Delete {
        owner_guid: u64,
        quest_id: u32,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerQuestSeasonalCompletionPersistenceLikeCpp {
    pub quest_id: u32,
    pub event_id: u16,
    pub completed_time: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlayerQuestLockoutPersistenceRequestLikeCpp {
    Daily {
        owner_guid: u64,
        completed_time: i64,
        quest_ids: Vec<u32>,
    },
    Weekly {
        owner_guid: u64,
        quest_ids: Vec<u32>,
    },
    Monthly {
        owner_guid: u64,
        quest_ids: Vec<u32>,
    },
    Seasonal {
        owner_guid: u64,
        completions: Vec<PlayerQuestSeasonalCompletionPersistenceLikeCpp>,
    },
}

pub trait PlayerQuestPersistencePortLikeCpp: Send + Sync {
    fn load_active_statuses_like_cpp(
        &self,
        owner_guid: u64,
    ) -> PersistenceFutureLikeCpp<
        '_,
        PlayerQuestLoadOutcomeLikeCpp<PlayerQuestActivePersistenceRowLikeCpp>,
    >;

    fn load_objectives_like_cpp(
        &self,
        owner_guid: u64,
    ) -> PersistenceFutureLikeCpp<
        '_,
        PlayerQuestLoadOutcomeLikeCpp<PlayerQuestObjectivePersistenceRowLikeCpp>,
    >;

    fn load_rewarded_like_cpp(
        &self,
        owner_guid: u64,
    ) -> PersistenceFutureLikeCpp<
        '_,
        PlayerQuestLoadOutcomeLikeCpp<PlayerQuestIdPersistenceRowLikeCpp>,
    >;

    fn load_daily_like_cpp(
        &self,
        owner_guid: u64,
    ) -> PersistenceFutureLikeCpp<
        '_,
        PlayerQuestLoadOutcomeLikeCpp<PlayerQuestDailyPersistenceRowLikeCpp>,
    >;

    fn load_weekly_like_cpp(
        &self,
        owner_guid: u64,
    ) -> PersistenceFutureLikeCpp<
        '_,
        PlayerQuestLoadOutcomeLikeCpp<PlayerQuestIdPersistenceRowLikeCpp>,
    >;

    fn load_monthly_like_cpp(
        &self,
        owner_guid: u64,
    ) -> PersistenceFutureLikeCpp<
        '_,
        PlayerQuestLoadOutcomeLikeCpp<PlayerQuestIdPersistenceRowLikeCpp>,
    >;

    fn load_seasonal_like_cpp(
        &self,
        owner_guid: u64,
    ) -> PersistenceFutureLikeCpp<
        '_,
        PlayerQuestLoadOutcomeLikeCpp<PlayerQuestSeasonalPersistenceRowLikeCpp>,
    >;

    fn persist_status_like_cpp(
        &self,
        request: PlayerQuestStatusPersistenceRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'_, PersistenceOutcomeLikeCpp>;

    fn persist_lockout_like_cpp(
        &self,
        request: PlayerQuestLockoutPersistenceRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'_, PersistenceOutcomeLikeCpp>;
}
