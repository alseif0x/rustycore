// Copyright (c) 2026 alseif0x
// RustyCore - WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 - https://www.gnu.org/licenses/gpl-3.0.html

//! C++ `ObjectMgr::LoadGameObjectQuestItems` / `LoadCreatureQuestItems`.

use std::collections::HashMap;

use anyhow::Result;
use wow_database::{WorldDatabase, WorldStatements};

use crate::DifficultyStore;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GameObjectQuestItemRowLikeCpp {
    pub gameobject_entry: u32,
    pub item_id: u32,
    pub idx: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CreatureQuestItemRowLikeCpp {
    pub creature_entry: u32,
    pub difficulty_id: u8,
    pub item_id: u32,
    pub idx: u32,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct GameObjectQuestItemLoadReportLikeCpp {
    pub rows_seen: usize,
    pub loaded_items: usize,
    pub skipped_missing_gameobject: Vec<(u32, u32)>,
    pub skipped_missing_item: Vec<(u32, u32, u32)>,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct CreatureQuestItemLoadReportLikeCpp {
    pub rows_seen: usize,
    pub loaded_items: usize,
    pub skipped_missing_creature: Vec<(u32, u8, u32)>,
    pub skipped_missing_item: Vec<(u32, u8, u32, u32)>,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct GameObjectQuestItemStoreLikeCpp {
    items_by_entry: HashMap<u32, Vec<u32>>,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct CreatureQuestItemStoreLikeCpp {
    items_by_entry_and_difficulty: HashMap<(u32, u8), Vec<u32>>,
}

pub struct GameObjectQuestItemLoadOutcomeLikeCpp {
    pub store: GameObjectQuestItemStoreLikeCpp,
    pub report: GameObjectQuestItemLoadReportLikeCpp,
}

pub struct CreatureQuestItemLoadOutcomeLikeCpp {
    pub store: CreatureQuestItemStoreLikeCpp,
    pub report: CreatureQuestItemLoadReportLikeCpp,
}

impl GameObjectQuestItemStoreLikeCpp {
    pub fn from_rows_like_cpp(
        rows: impl IntoIterator<Item = GameObjectQuestItemRowLikeCpp>,
        gameobject_exists: impl Fn(u32) -> bool,
        item_exists: impl Fn(u32) -> bool,
    ) -> GameObjectQuestItemLoadOutcomeLikeCpp {
        let mut items_by_entry: HashMap<u32, Vec<u32>> = HashMap::new();
        let mut report = GameObjectQuestItemLoadReportLikeCpp::default();

        for row in rows {
            report.rows_seen += 1;

            if !gameobject_exists(row.gameobject_entry) {
                report
                    .skipped_missing_gameobject
                    .push((row.gameobject_entry, row.idx));
                continue;
            }

            if !item_exists(row.item_id) {
                report
                    .skipped_missing_item
                    .push((row.gameobject_entry, row.item_id, row.idx));
                continue;
            }

            items_by_entry
                .entry(row.gameobject_entry)
                .or_default()
                .push(row.item_id);
            report.loaded_items += 1;
        }

        GameObjectQuestItemLoadOutcomeLikeCpp {
            store: Self { items_by_entry },
            report,
        }
    }

    pub async fn load_like_cpp(
        db: &WorldDatabase,
        gameobject_exists: impl Fn(u32) -> bool,
        item_exists: impl Fn(u32) -> bool,
    ) -> Result<GameObjectQuestItemLoadOutcomeLikeCpp> {
        let mut result = db
            .query(&db.prepare(WorldStatements::SEL_GAMEOBJECT_QUEST_ITEM_ROWS))
            .await?;
        let mut rows = Vec::new();

        if !result.is_empty() {
            loop {
                rows.push(GameObjectQuestItemRowLikeCpp {
                    gameobject_entry: result.read(0),
                    item_id: result.read(1),
                    idx: result.read(2),
                });

                if !result.next_row() {
                    break;
                }
            }
        }

        Ok(Self::from_rows_like_cpp(
            rows,
            gameobject_exists,
            item_exists,
        ))
    }

    pub fn get_gameobject_quest_item_list_like_cpp(&self, entry: u32) -> Option<&[u32]> {
        self.items_by_entry.get(&entry).map(Vec::as_slice)
    }

    pub fn len(&self) -> usize {
        self.items_by_entry.values().map(Vec::len).sum()
    }

    pub fn is_empty(&self) -> bool {
        self.items_by_entry.is_empty()
    }
}

impl CreatureQuestItemStoreLikeCpp {
    pub fn from_rows_like_cpp(
        rows: impl IntoIterator<Item = CreatureQuestItemRowLikeCpp>,
        creature_exists: impl Fn(u32) -> bool,
        item_exists: impl Fn(u32) -> bool,
    ) -> CreatureQuestItemLoadOutcomeLikeCpp {
        let mut items_by_entry_and_difficulty: HashMap<(u32, u8), Vec<u32>> = HashMap::new();
        let mut report = CreatureQuestItemLoadReportLikeCpp::default();

        for row in rows {
            report.rows_seen += 1;

            if !creature_exists(row.creature_entry) {
                report.skipped_missing_creature.push((
                    row.creature_entry,
                    row.difficulty_id,
                    row.idx,
                ));
                continue;
            }

            if !item_exists(row.item_id) {
                report.skipped_missing_item.push((
                    row.creature_entry,
                    row.difficulty_id,
                    row.item_id,
                    row.idx,
                ));
                continue;
            }

            items_by_entry_and_difficulty
                .entry((row.creature_entry, row.difficulty_id))
                .or_default()
                .push(row.item_id);
            report.loaded_items += 1;
        }

        CreatureQuestItemLoadOutcomeLikeCpp {
            store: Self {
                items_by_entry_and_difficulty,
            },
            report,
        }
    }

    pub async fn load_like_cpp(
        db: &WorldDatabase,
        creature_exists: impl Fn(u32) -> bool,
        item_exists: impl Fn(u32) -> bool,
    ) -> Result<CreatureQuestItemLoadOutcomeLikeCpp> {
        let mut result = db
            .query(&db.prepare(WorldStatements::SEL_CREATURE_QUEST_ITEM_ROWS))
            .await?;
        let mut rows = Vec::new();

        if !result.is_empty() {
            loop {
                rows.push(CreatureQuestItemRowLikeCpp {
                    creature_entry: result.read(0),
                    difficulty_id: result.read(1),
                    item_id: result.read(2),
                    idx: result.read(3),
                });

                if !result.next_row() {
                    break;
                }
            }
        }

        Ok(Self::from_rows_like_cpp(rows, creature_exists, item_exists))
    }

    pub fn get_creature_quest_item_list_like_cpp(
        &self,
        creature_entry: u32,
        difficulty_id: u8,
        difficulty_store: &DifficultyStore,
    ) -> Option<&[u32]> {
        self.get_creature_quest_item_list_inner_like_cpp(
            creature_entry,
            difficulty_id,
            difficulty_store,
            0,
        )
    }

    fn get_creature_quest_item_list_inner_like_cpp(
        &self,
        creature_entry: u32,
        difficulty_id: u8,
        difficulty_store: &DifficultyStore,
        depth: u8,
    ) -> Option<&[u32]> {
        if let Some(items) = self
            .items_by_entry_and_difficulty
            .get(&(creature_entry, difficulty_id))
        {
            return Some(items.as_slice());
        }

        let difficulty = difficulty_store.get(u32::from(difficulty_id))?;
        if difficulty.fallback_difficulty_id == difficulty_id || depth > 32 {
            return None;
        }

        self.get_creature_quest_item_list_inner_like_cpp(
            creature_entry,
            difficulty.fallback_difficulty_id,
            difficulty_store,
            depth + 1,
        )
    }

    pub fn len(&self) -> usize {
        self.items_by_entry_and_difficulty
            .values()
            .map(Vec::len)
            .sum()
    }

    pub fn is_empty(&self) -> bool {
        self.items_by_entry_and_difficulty.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::DifficultyEntry;

    fn go_row(entry: u32, item_id: u32, idx: u32) -> GameObjectQuestItemRowLikeCpp {
        GameObjectQuestItemRowLikeCpp {
            gameobject_entry: entry,
            item_id,
            idx,
        }
    }

    fn creature_row(
        entry: u32,
        difficulty_id: u8,
        item_id: u32,
        idx: u32,
    ) -> CreatureQuestItemRowLikeCpp {
        CreatureQuestItemRowLikeCpp {
            creature_entry: entry,
            difficulty_id,
            item_id,
            idx,
        }
    }

    #[test]
    fn gameobject_quest_items_keep_cpp_idx_order_and_skip_invalid_refs() {
        let outcome = GameObjectQuestItemStoreLikeCpp::from_rows_like_cpp(
            [
                go_row(10, 100, 1),
                go_row(10, 101, 2),
                go_row(99, 100, 3),
                go_row(10, 999, 4),
            ],
            |entry| entry == 10,
            |item| item < 900,
        );

        assert_eq!(outcome.report.rows_seen, 4);
        assert_eq!(outcome.report.loaded_items, 2);
        assert_eq!(outcome.report.skipped_missing_gameobject, [(99, 3)]);
        assert_eq!(outcome.report.skipped_missing_item, [(10, 999, 4)]);
        assert_eq!(
            outcome.store.get_gameobject_quest_item_list_like_cpp(10),
            Some([100, 101].as_slice())
        );
    }

    #[test]
    fn creature_quest_items_keep_cpp_idx_order_and_skip_invalid_refs() {
        let outcome = CreatureQuestItemStoreLikeCpp::from_rows_like_cpp(
            [
                creature_row(20, 1, 200, 1),
                creature_row(20, 1, 201, 2),
                creature_row(99, 1, 200, 3),
                creature_row(20, 1, 999, 4),
            ],
            |entry| entry == 20,
            |item| item < 900,
        );

        assert_eq!(outcome.report.rows_seen, 4);
        assert_eq!(outcome.report.loaded_items, 2);
        assert_eq!(outcome.report.skipped_missing_creature, [(99, 1, 3)]);
        assert_eq!(outcome.report.skipped_missing_item, [(20, 1, 999, 4)]);
        let difficulties = DifficultyStore::from_ids([1]);
        assert_eq!(
            outcome
                .store
                .get_creature_quest_item_list_like_cpp(20, 1, &difficulties),
            Some([200, 201].as_slice())
        );
    }

    #[test]
    fn creature_quest_items_follow_cpp_difficulty_fallback() {
        let outcome = CreatureQuestItemStoreLikeCpp::from_rows_like_cpp(
            [creature_row(30, 1, 300, 1)],
            |_| true,
            |_| true,
        );
        let difficulties = DifficultyStore::from_entries([
            DifficultyEntry {
                id: 2,
                instance_type: 0,
                flags: 0,
                fallback_difficulty_id: 1,
                toggle_difficulty_id: 0,
            },
            DifficultyEntry {
                id: 1,
                instance_type: 0,
                flags: 0,
                fallback_difficulty_id: 0,
                toggle_difficulty_id: 0,
            },
        ]);

        assert_eq!(
            outcome
                .store
                .get_creature_quest_item_list_like_cpp(30, 2, &difficulties),
            Some([300].as_slice())
        );
    }
}
