//! SQLx-free World source contract for immutable player base stats.

use crate::PersistenceFutureLikeCpp;

pub const PLAYER_PRIMARY_STAT_COUNT_LIKE_CPP: usize = 5;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerRaceStatsPersistenceRowLikeCpp {
    pub race: u8,
    pub stat_modifiers: [i16; PLAYER_PRIMARY_STAT_COUNT_LIKE_CPP],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerClassLevelStatsPersistenceRowLikeCpp {
    pub class: u8,
    pub level: u8,
    pub primary_stats: [u16; PLAYER_PRIMARY_STAT_COUNT_LIKE_CPP],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlayerBaseStatsLoadOutcomeLikeCpp<T> {
    Loaded(Vec<T>),
    Failed { reason: String },
}

/// C++ `ObjectMgr::LoadPlayerInfo` World source for immutable base stats.
///
/// The two operations are intentionally staged rather than table CRUD: C++
/// aborts on an empty race batch before issuing the class/level query. Domain
/// validation stays between the stages and final publication remains atomic.
pub trait PlayerBaseStatsPersistencePortLikeCpp: Send + Sync {
    fn load_player_race_stats_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        PlayerBaseStatsLoadOutcomeLikeCpp<PlayerRaceStatsPersistenceRowLikeCpp>,
    >;

    fn load_player_class_level_stats_rows_like_cpp(
        &self,
    ) -> PersistenceFutureLikeCpp<
        '_,
        PlayerBaseStatsLoadOutcomeLikeCpp<PlayerClassLevelStatsPersistenceRowLikeCpp>,
    >;
}
