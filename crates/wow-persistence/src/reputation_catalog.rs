//! SQLx-free startup source contract for the C++ reputation catalogs.

use crate::PersistenceFutureLikeCpp;

pub const REPUTATION_SPILLOVER_SLOT_COUNT_LIKE_CPP: usize = 5;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ReputationRewardRatePersistenceRowLikeCpp {
    pub faction_id: u32,
    pub quest_rate: f32,
    pub quest_daily_rate: f32,
    pub quest_weekly_rate: f32,
    pub quest_monthly_rate: f32,
    pub quest_repeatable_rate: f32,
    pub creature_rate: f32,
    pub spell_rate: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CreatureOnKillReputationPersistenceRowLikeCpp {
    pub creature_id: u32,
    pub rep_faction_1: u32,
    pub rep_faction_2: u32,
    pub is_team_award_1: bool,
    pub reputation_max_cap_1: u8,
    pub rep_value_1: i32,
    pub is_team_award_2: bool,
    pub reputation_max_cap_2: u8,
    pub rep_value_2: i32,
    pub team_dependent: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ReputationSpilloverTemplatePersistenceRowLikeCpp {
    pub faction_id: u32,
    pub faction: [u32; REPUTATION_SPILLOVER_SLOT_COUNT_LIKE_CPP],
    pub faction_rate: [f32; REPUTATION_SPILLOVER_SLOT_COUNT_LIKE_CPP],
    pub faction_rank: [u8; REPUTATION_SPILLOVER_SLOT_COUNT_LIKE_CPP],
}

#[derive(Debug, Clone, PartialEq)]
pub enum ReputationCatalogLoadOutcomeLikeCpp<T> {
    Loaded(Vec<T>),
    Failed { reason: String },
}

/// C++ `ObjectMgr` reputation world-table source. The concrete adapter owns
/// statement identity and row decoding; the data owner retains validation,
/// duplicate replacement and the immutable stores.
pub trait ReputationCatalogPersistencePortLikeCpp: Send + Sync {
    fn load_reward_rate_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        ReputationCatalogLoadOutcomeLikeCpp<ReputationRewardRatePersistenceRowLikeCpp>,
    >;

    fn load_creature_onkill_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        ReputationCatalogLoadOutcomeLikeCpp<CreatureOnKillReputationPersistenceRowLikeCpp>,
    >;

    fn load_spillover_template_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        ReputationCatalogLoadOutcomeLikeCpp<ReputationSpilloverTemplatePersistenceRowLikeCpp>,
    >;
}
