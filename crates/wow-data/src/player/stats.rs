// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3

//! C++ `ObjectMgr::LoadPlayerInfo` player stat data.
//!
//! Primary stats come from `player_classlevelstats` plus the signed
//! `player_racestats` modifiers. Base mana comes from the client
//! `gt/BaseMp.txt` GameTable through `ObjectMgr::GetPlayerClassLevelInfo`.
//! C++ does not consume the legacy C# `player_levelstats.basehp/basemana`
//! projection.

pub use wow_data_model::player_stats::{
    PlayerLevelStats, PlayerSpellBonusInputLikeCpp, PlayerStatSystemInputLikeCpp,
    PlayerStatSystemProjectionLikeCpp,
};

/// C++ `CONFIG_STATS_LIMITS_*` (`World.cpp:1664-1668`, defaults `false`/`95.0`):
/// `Player::UpdateBlockPercentage`, `UpdateDodgePercentage`,
/// `UpdateParryPercentage` and `UpdateCritPercentage` cap their published
/// percentage when the limit is enabled.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StatsLimitsLikeCpp {
    pub enabled: bool,
    pub dodge: f32,
    pub parry: f32,
    pub block: f32,
    pub crit: f32,
}

impl Default for StatsLimitsLikeCpp {
    fn default() -> Self {
        Self {
            enabled: false,
            dodge: 95.0,
            parry: 95.0,
            block: 95.0,
            crit: 95.0,
        }
    }
}

impl StatsLimitsLikeCpp {
    /// C++ `value = value > limit ? limit : value` for the dodge limit.
    pub fn clamp_dodge_like_cpp(&self, value: f32) -> f32 {
        self.clamp(value, self.dodge)
    }

    /// Same cap for the parry limit.
    pub fn clamp_parry_like_cpp(&self, value: f32) -> f32 {
        self.clamp(value, self.parry)
    }

    /// Same cap for the block limit.
    pub fn clamp_block_like_cpp(&self, value: f32) -> f32 {
        self.clamp(value, self.block)
    }

    /// Same cap for the crit limit; C++ applies it to the main-hand, off-hand
    /// and ranged crit percentages through one `applyCritLimit` lambda.
    pub fn clamp_crit_like_cpp(&self, value: f32) -> f32 {
        self.clamp(value, self.crit)
    }

    fn clamp(&self, value: f32, limit: f32) -> f32 {
        if self.enabled && value > limit {
            limit
        } else {
            value
        }
    }
}

mod calculation;
mod store;

pub use calculation::effective_weapon_damage_ranges_like_cpp;
pub use store::{
    PlayerClassLevelStatsRowLikeCpp, PlayerClassLevelStatsRowsLikeCpp, PlayerRaceStatsRowLikeCpp,
    PlayerRaceStatsRowsLikeCpp, PlayerStatsStore,
};

#[cfg(test)]
#[path = "stats/tests/mod.rs"]
mod tests;
