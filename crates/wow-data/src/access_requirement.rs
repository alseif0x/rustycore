// Copyright (c) 2026 alseif0x
// RustyCore - WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 - https://www.gnu.org/licenses/gpl-3.0.html

//! C++ `ObjectMgr::LoadAccessRequirements` data model.

use std::collections::HashMap;

use anyhow::Result;
use tracing::warn;
use wow_database::WorldDatabase;

use crate::{Db2IdStore, ItemStore, MapDifficultyStore, MapStore, quest::QuestStore};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccessRequirementLikeCpp {
    pub map_id: u32,
    pub difficulty: u8,
    pub level_min: u8,
    pub level_max: u8,
    pub item: u32,
    pub item2: u32,
    pub quest_done_a: u32,
    pub quest_done_h: u32,
    pub completed_achievement: u32,
    pub quest_failed_text: String,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct AccessRequirementStoreLikeCpp {
    by_map_difficulty: HashMap<(u32, u8), AccessRequirementLikeCpp>,
}

impl AccessRequirementStoreLikeCpp {
    pub fn from_entries_like_cpp(
        entries: impl IntoIterator<Item = AccessRequirementLikeCpp>,
    ) -> Self {
        Self {
            by_map_difficulty: entries
                .into_iter()
                .map(|entry| ((entry.map_id, entry.difficulty), entry))
                .collect(),
        }
    }

    /// Load `access_requirement` using the exact C++ selected columns.
    ///
    /// C++ anchor:
    /// `/home/server/woltk-trinity-legacy/src/server/game/Globals/ObjectMgr.cpp:7180-7270`.
    pub async fn load_like_cpp(
        db: &WorldDatabase,
        map_store: &MapStore,
        map_difficulty_store: &MapDifficultyStore,
        item_store: &ItemStore,
        quest_store: &QuestStore,
        achievement_store: &Db2IdStore,
    ) -> Result<Self> {
        let mut result = db
            .direct_query(
                "SELECT mapid, difficulty, level_min, level_max, item, item2, quest_done_A, quest_done_H, completed_achievement, quest_failed_text FROM access_requirement",
            )
            .await?;
        if result.is_empty() {
            return Ok(Self::default());
        }

        let mut entries = Vec::with_capacity(result.row_count_like_cpp());
        loop {
            let fields = result.fields();
            let map_id = fields.try_read::<u32>(0).unwrap_or(0);
            if map_store.get(map_id).is_none() {
                warn!(
                    target: "sql.sql",
                    "Map {map_id} referenced in `access_requirement` does not exist, skipped."
                );
                if !result.next_row() {
                    break;
                }
                continue;
            }

            let difficulty = fields.try_read::<u8>(1).unwrap_or(0);
            if map_difficulty_store.get(map_id, difficulty).is_none() {
                warn!(
                    target: "sql.sql",
                    "Map {map_id} referenced in `access_requirement` does not have difficulty {difficulty}, skipped"
                );
                if !result.next_row() {
                    break;
                }
                continue;
            }

            let mut entry = AccessRequirementLikeCpp {
                map_id,
                difficulty,
                level_min: fields.try_read::<u8>(2).unwrap_or(0),
                level_max: fields.try_read::<u8>(3).unwrap_or(0),
                item: fields.try_read::<u32>(4).unwrap_or(0),
                item2: fields.try_read::<u32>(5).unwrap_or(0),
                quest_done_a: fields.try_read::<u32>(6).unwrap_or(0),
                quest_done_h: fields.try_read::<u32>(7).unwrap_or(0),
                completed_achievement: fields.try_read::<u32>(8).unwrap_or(0),
                quest_failed_text: fields.read_string(9),
            };

            if entry.item != 0 && item_store.get(entry.item).is_none() {
                warn!(
                    target: "sql.sql",
                    "Key item {} does not exist for map {} difficulty {}, removing key requirement.",
                    entry.item, map_id, difficulty
                );
                entry.item = 0;
            }
            if entry.item2 != 0 && item_store.get(entry.item2).is_none() {
                warn!(
                    target: "sql.sql",
                    "Second item {} does not exist for map {} difficulty {}, removing key requirement.",
                    entry.item2, map_id, difficulty
                );
                entry.item2 = 0;
            }
            if entry.quest_done_a != 0 && quest_store.get(entry.quest_done_a).is_none() {
                warn!(
                    target: "sql.sql",
                    "Required Alliance Quest {} not exist for map {} difficulty {}, remove quest done requirement.",
                    entry.quest_done_a, map_id, difficulty
                );
                entry.quest_done_a = 0;
            }
            if entry.quest_done_h != 0 && quest_store.get(entry.quest_done_h).is_none() {
                warn!(
                    target: "sql.sql",
                    "Required Horde Quest {} not exist for map {} difficulty {}, remove quest done requirement.",
                    entry.quest_done_h, map_id, difficulty
                );
                entry.quest_done_h = 0;
            }
            if entry.completed_achievement != 0
                && !achievement_store.contains(entry.completed_achievement)
            {
                warn!(
                    target: "sql.sql",
                    "Required Achievement {} not exist for map {} difficulty {}, remove quest done requirement.",
                    entry.completed_achievement, map_id, difficulty
                );
                entry.completed_achievement = 0;
            }

            entries.push(entry);
            if !result.next_row() {
                break;
            }
        }

        Ok(Self::from_entries_like_cpp(entries))
    }

    pub fn get(&self, map_id: u32, difficulty: u8) -> Option<&AccessRequirementLikeCpp> {
        self.by_map_difficulty.get(&(map_id, difficulty))
    }

    pub fn len(&self) -> usize {
        self.by_map_difficulty.len()
    }

    pub fn is_empty(&self) -> bool {
        self.by_map_difficulty.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn access_requirement_store_indexes_by_map_and_difficulty_like_cpp() {
        let store = AccessRequirementStoreLikeCpp::from_entries_like_cpp([
            AccessRequirementLikeCpp {
                map_id: 631,
                difficulty: 3,
                level_min: 80,
                level_max: 0,
                item: 0,
                item2: 0,
                quest_done_a: 0,
                quest_done_h: 0,
                completed_achievement: 0,
                quest_failed_text: String::new(),
            },
            AccessRequirementLikeCpp {
                map_id: 631,
                difficulty: 4,
                level_min: 0,
                level_max: 0,
                item: 999,
                item2: 0,
                quest_done_a: 0,
                quest_done_h: 0,
                completed_achievement: 0,
                quest_failed_text: String::new(),
            },
        ]);

        assert_eq!(store.get(631, 3).unwrap().level_min, 80);
        assert_eq!(store.get(631, 4).unwrap().item, 999);
        assert!(store.get(631, 5).is_none());
        assert_eq!(store.len(), 2);
    }
}
