// Copyright (c) 2026 alseif0x
// RustyCore - WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 - https://www.gnu.org/licenses/gpl-3.0.html

//! C++ `ObjectMgr::LoadExplorationBaseXP` data model.

use std::collections::BTreeMap;

use anyhow::Result;
use tracing::info;
use wow_database::{WorldDatabase, WorldStatements};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExplorationBaseXpRowLikeCpp {
    pub level: u8,
    pub base_xp: u32,
}

/// Represented C++ `ObjectMgr::_baseXPTable`.
#[derive(Debug, Clone, Default)]
pub struct ExplorationBaseXpStoreLikeCpp {
    base_xp_by_level: BTreeMap<u8, u32>,
}

impl ExplorationBaseXpStoreLikeCpp {
    pub fn from_rows_like_cpp(rows: impl IntoIterator<Item = ExplorationBaseXpRowLikeCpp>) -> Self {
        let mut base_xp_by_level = BTreeMap::new();
        for row in rows {
            base_xp_by_level.insert(row.level, row.base_xp);
        }

        Self { base_xp_by_level }
    }

    /// C++ `ObjectMgr::LoadExplorationBaseXP`.
    pub async fn load_like_cpp(db: &WorldDatabase) -> Result<Self> {
        let stmt = db.prepare(WorldStatements::SEL_EXPLORATION_BASE_XP);
        let mut result = db.query(&stmt).await?;
        let mut rows = Vec::new();

        if !result.is_empty() {
            loop {
                rows.push(ExplorationBaseXpRowLikeCpp {
                    level: result.read(0),
                    base_xp: result.read(1),
                });

                if !result.next_row() {
                    break;
                }
            }
        }

        let store = Self::from_rows_like_cpp(rows);
        info!("Loaded {} BaseXP definitions", store.len());
        Ok(store)
    }

    /// C++ `ObjectMgr::GetBaseXP`.
    pub fn base_xp_like_cpp(&self, level: u8) -> u32 {
        self.base_xp_by_level.get(&level).copied().unwrap_or(0)
    }

    pub fn len(&self) -> usize {
        self.base_xp_by_level.len()
    }

    pub fn is_empty(&self) -> bool {
        self.base_xp_by_level.is_empty()
    }

    /// C++ `Player::CheckAreaExploreAndOutdoor` exploration XP calculation.
    pub fn exploration_xp_reward_like_cpp(
        &self,
        player_level: u8,
        exploration_level: i8,
        rate_xp_explore: f32,
        min_discovered_scaled_xp_ratio: u32,
    ) -> u32 {
        if exploration_level <= 0 {
            return 0;
        }

        let exploration_level_u8 = exploration_level as u8;
        let diff = i32::from(player_level) - i32::from(exploration_level);
        let base_xp = if diff < -5 {
            self.base_xp_like_cpp(player_level.saturating_add(5))
        } else if diff > 5 {
            let exploration_percent = (100 - ((diff - 5) * 5)).max(0) as u32;
            self.base_xp_like_cpp(exploration_level_u8)
                .saturating_mul(exploration_percent)
                / 100
        } else {
            self.base_xp_like_cpp(exploration_level_u8)
        };

        let mut xp = (base_xp as f32 * rate_xp_explore) as u32;
        if min_discovered_scaled_xp_ratio != 0 {
            let min_scaled_xp = ((self.base_xp_like_cpp(exploration_level_u8) as f32
                * rate_xp_explore) as u32)
                .saturating_mul(min_discovered_scaled_xp_ratio)
                / 100;
            xp = xp.max(min_scaled_xp);
        }

        xp
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(level: u8, base_xp: u32) -> ExplorationBaseXpRowLikeCpp {
        ExplorationBaseXpRowLikeCpp { level, base_xp }
    }

    #[test]
    fn exploration_base_xp_missing_level_returns_zero_like_cpp() {
        let store = ExplorationBaseXpStoreLikeCpp::from_rows_like_cpp([row(10, 85)]);

        assert_eq!(store.base_xp_like_cpp(10), 85);
        assert_eq!(store.base_xp_like_cpp(11), 0);
    }

    #[test]
    fn exploration_base_xp_duplicate_level_overwrites_like_cpp() {
        let store = ExplorationBaseXpStoreLikeCpp::from_rows_like_cpp([
            row(12, 100),
            row(12, 150),
            row(13, 0),
        ]);

        assert_eq!(store.len(), 2);
        assert_eq!(store.base_xp_like_cpp(12), 150);
        assert_eq!(store.base_xp_like_cpp(13), 0);
    }

    #[test]
    fn exploration_xp_reward_matches_cpp_level_branches() {
        let store = ExplorationBaseXpStoreLikeCpp::from_rows_like_cpp([
            row(10, 100),
            row(15, 200),
            row(20, 400),
        ]);

        assert_eq!(store.exploration_xp_reward_like_cpp(20, 10, 1.0, 0), 75);
        assert_eq!(store.exploration_xp_reward_like_cpp(10, 20, 1.0, 0), 200);
        assert_eq!(store.exploration_xp_reward_like_cpp(15, 15, 1.5, 0), 300);
        assert_eq!(store.exploration_xp_reward_like_cpp(20, 10, 1.0, 50), 75);
        assert_eq!(store.exploration_xp_reward_like_cpp(50, 10, 1.0, 50), 50);
        assert_eq!(store.exploration_xp_reward_like_cpp(20, 0, 1.0, 0), 0);
    }
}
