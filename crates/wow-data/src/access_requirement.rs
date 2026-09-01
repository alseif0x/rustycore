// Copyright (c) 2026 alseif0x
// RustyCore - WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 - https://www.gnu.org/licenses/gpl-3.0.html

//! C++ `ObjectMgr::LoadAccessRequirements` data model.

use std::collections::HashMap;

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccessRequirementRowLikeCpp {
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
pub struct AccessRequirementLoadReportLikeCpp {
    pub rows_seen: usize,
    pub loaded_rows: usize,
    pub skipped_missing_map: Vec<u32>,
    pub skipped_missing_difficulty: Vec<(u32, u8)>,
    pub cleared_missing_item: Vec<(u32, u8, u32)>,
    pub cleared_missing_item2: Vec<(u32, u8, u32)>,
    pub cleared_missing_quest_a: Vec<(u32, u8, u32)>,
    pub cleared_missing_quest_h: Vec<(u32, u8, u32)>,
    pub cleared_missing_achievement: Vec<(u32, u8, u32)>,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct AccessRequirementStoreLikeCpp {
    by_map_difficulty: HashMap<(u32, u8), AccessRequirementLikeCpp>,
}

pub struct AccessRequirementLoadOutcomeLikeCpp {
    pub store: AccessRequirementStoreLikeCpp,
    pub report: AccessRequirementLoadReportLikeCpp,
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

    pub fn from_rows_like_cpp(
        rows: impl IntoIterator<Item = AccessRequirementRowLikeCpp>,
        mut map_exists: impl FnMut(u32) -> bool,
        mut map_difficulty_exists: impl FnMut(u32, u8) -> bool,
        mut item_exists: impl FnMut(u32) -> bool,
        mut quest_exists: impl FnMut(u32) -> bool,
        mut achievement_exists: impl FnMut(u32) -> bool,
    ) -> AccessRequirementLoadOutcomeLikeCpp {
        let mut report = AccessRequirementLoadReportLikeCpp::default();
        let mut entries = Vec::new();

        for row in rows {
            report.rows_seen += 1;

            if !map_exists(row.map_id) {
                report.skipped_missing_map.push(row.map_id);
                continue;
            }

            if !map_difficulty_exists(row.map_id, row.difficulty) {
                report
                    .skipped_missing_difficulty
                    .push((row.map_id, row.difficulty));
                continue;
            }

            let mut entry = AccessRequirementLikeCpp {
                map_id: row.map_id,
                difficulty: row.difficulty,
                level_min: row.level_min,
                level_max: row.level_max,
                item: row.item,
                item2: row.item2,
                quest_done_a: row.quest_done_a,
                quest_done_h: row.quest_done_h,
                completed_achievement: row.completed_achievement,
                quest_failed_text: row.quest_failed_text,
            };

            if entry.item != 0 && !item_exists(entry.item) {
                report
                    .cleared_missing_item
                    .push((entry.map_id, entry.difficulty, entry.item));
                entry.item = 0;
            }
            if entry.item2 != 0 && !item_exists(entry.item2) {
                report
                    .cleared_missing_item2
                    .push((entry.map_id, entry.difficulty, entry.item2));
                entry.item2 = 0;
            }
            if entry.quest_done_a != 0 && !quest_exists(entry.quest_done_a) {
                report.cleared_missing_quest_a.push((
                    entry.map_id,
                    entry.difficulty,
                    entry.quest_done_a,
                ));
                entry.quest_done_a = 0;
            }
            if entry.quest_done_h != 0 && !quest_exists(entry.quest_done_h) {
                report.cleared_missing_quest_h.push((
                    entry.map_id,
                    entry.difficulty,
                    entry.quest_done_h,
                ));
                entry.quest_done_h = 0;
            }
            if entry.completed_achievement != 0 && !achievement_exists(entry.completed_achievement)
            {
                report.cleared_missing_achievement.push((
                    entry.map_id,
                    entry.difficulty,
                    entry.completed_achievement,
                ));
                entry.completed_achievement = 0;
            }

            entries.push(entry);
            report.loaded_rows += 1;
        }

        AccessRequirementLoadOutcomeLikeCpp {
            store: Self::from_entries_like_cpp(entries),
            report,
        }
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

    #[test]
    fn access_requirement_rows_validate_refs_like_cpp() {
        let outcome = AccessRequirementStoreLikeCpp::from_rows_like_cpp(
            [
                AccessRequirementRowLikeCpp {
                    map_id: 631,
                    difficulty: 3,
                    level_min: 80,
                    level_max: 0,
                    item: 100,
                    item2: 101,
                    quest_done_a: 200,
                    quest_done_h: 201,
                    completed_achievement: 300,
                    quest_failed_text: "locked".to_string(),
                },
                AccessRequirementRowLikeCpp {
                    map_id: 999,
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
                AccessRequirementRowLikeCpp {
                    map_id: 631,
                    difficulty: 9,
                    level_min: 80,
                    level_max: 0,
                    item: 0,
                    item2: 0,
                    quest_done_a: 0,
                    quest_done_h: 0,
                    completed_achievement: 0,
                    quest_failed_text: String::new(),
                },
            ],
            |map_id| map_id == 631,
            |map_id, difficulty| map_id == 631 && difficulty == 3,
            |item_id| item_id == 100,
            |quest_id| quest_id == 200,
            |achievement_id| achievement_id == 300,
        );

        assert_eq!(outcome.report.rows_seen, 3);
        assert_eq!(outcome.report.loaded_rows, 1);
        assert_eq!(outcome.report.skipped_missing_map, vec![999]);
        assert_eq!(outcome.report.skipped_missing_difficulty, vec![(631, 9)]);
        assert_eq!(
            outcome.report.cleared_missing_item,
            Vec::<(u32, u8, u32)>::new()
        );
        assert_eq!(outcome.report.cleared_missing_item2, vec![(631, 3, 101)]);
        assert_eq!(
            outcome.report.cleared_missing_quest_a,
            Vec::<(u32, u8, u32)>::new()
        );
        assert_eq!(outcome.report.cleared_missing_quest_h, vec![(631, 3, 201)]);
        assert_eq!(
            outcome.report.cleared_missing_achievement,
            Vec::<(u32, u8, u32)>::new()
        );

        let entry = outcome.store.get(631, 3).unwrap();
        assert_eq!(entry.level_min, 80);
        assert_eq!(entry.item, 100);
        assert_eq!(entry.item2, 0);
        assert_eq!(entry.quest_done_a, 200);
        assert_eq!(entry.quest_done_h, 0);
        assert_eq!(entry.completed_achievement, 300);
        assert_eq!(entry.quest_failed_text, "locked");
    }
}
