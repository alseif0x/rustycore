// Copyright (c) 2026 alseif0x
// RustyCore - WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 - https://www.gnu.org/licenses/gpl-3.0.html

//! C++ `LFGMgr::LoadLFGDungeons` / `LoadRewards` represented model.

use std::collections::{BTreeSet, HashMap};

use anyhow::Result;
use wow_database::{HotfixDatabase, HotfixStatements, WorldDatabase, WorldStatements};

use crate::{LfgDungeonsEntry, LfgDungeonsStore, MapDifficultyStore, MapStore, quest::QuestStore};

pub const LFG_FLAG_SEASONAL_LIKE_CPP: i32 = 0x4;
pub const LFG_TYPE_DUNGEON_LIKE_CPP: u8 = 1;
pub const LFG_TYPE_RAID_LIKE_CPP: u8 = 2;
pub const LFG_TYPE_HEROIC_LIKE_CPP: u8 = 5;
pub const LFG_TYPE_RANDOM_LIKE_CPP: u8 = 6;

#[derive(Debug, Clone, PartialEq)]
pub struct LfgDungeonDataLikeCpp {
    pub id: u32,
    pub name: String,
    pub map: u32,
    pub type_id: u8,
    pub expansion: u8,
    pub group: u8,
    pub min_level: u8,
    pub max_level: u8,
    pub difficulty: u8,
    pub seasonal: bool,
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub o: f32,
    pub required_item_level: u16,
    pub final_dungeon_encounter_id: u32,
}

impl LfgDungeonDataLikeCpp {
    pub fn from_db2_like_cpp(entry: &LfgDungeonsEntry) -> Option<Self> {
        let map = u32::try_from(entry.map_id).unwrap_or(0);
        let max_level = u8::try_from(entry.max_level).unwrap_or(u8::MAX);
        Some(Self {
            id: entry.id,
            name: entry.name.clone(),
            map,
            type_id: entry.type_id,
            expansion: entry.expansion_level,
            group: entry.group_id,
            min_level: entry.min_level,
            max_level,
            difficulty: entry.difficulty_id,
            seasonal: (entry.flags[0] & LFG_FLAG_SEASONAL_LIKE_CPP) != 0,
            x: 0.0,
            y: 0.0,
            z: 0.0,
            o: 0.0,
            required_item_level: 0,
            final_dungeon_encounter_id: u32::from(entry.final_encounter_id),
        })
    }

    /// C++ `LFGDungeonData::Entry`.
    pub fn entry_like_cpp(&self) -> u32 {
        self.id + (u32::from(self.type_id) << 24)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LfgDungeonTemplateRowLikeCpp {
    pub dungeon_id: u32,
    pub position_x: f32,
    pub position_y: f32,
    pub position_z: f32,
    pub orientation: f32,
    pub required_item_level: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LfgDungeonRewardLikeCpp {
    pub max_level: u8,
    pub first_quest_id: u32,
    pub other_quest_id: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LfgDungeonRewardRowLikeCpp {
    pub dungeon_id: u32,
    pub reward: LfgDungeonRewardLikeCpp,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LfgLoadReportLikeCpp {
    pub db2_rows_seen: usize,
    pub loaded_dungeons: usize,
    pub template_rows_seen: usize,
    pub loaded_templates: usize,
    pub reward_rows_seen: usize,
    pub loaded_rewards: usize,
    pub skipped_type: Vec<u32>,
    pub skipped_missing_map_difficulty: Vec<u32>,
    pub skipped_template_missing_dungeon: Vec<u32>,
    pub skipped_reward_missing_dungeon: Vec<u32>,
    pub skipped_reward_missing_first_quest: Vec<(u32, u32)>,
    pub corrected_reward_missing_other_quest: Vec<(u32, u32)>,
}

#[derive(Debug, Clone, Default)]
pub struct LfgDungeonStoreLikeCpp {
    dungeons: HashMap<u32, LfgDungeonDataLikeCpp>,
    cached_by_random: HashMap<u32, BTreeSet<u32>>,
    rewards: HashMap<u32, Vec<LfgDungeonRewardLikeCpp>>,
}

pub struct LfgLoadOutcomeLikeCpp {
    pub store: LfgDungeonStoreLikeCpp,
    pub report: LfgLoadReportLikeCpp,
}

impl LfgDungeonStoreLikeCpp {
    pub fn from_sources_like_cpp(
        db2_store: &LfgDungeonsStore,
        map_difficulty_store: &MapDifficultyStore,
        template_rows: impl IntoIterator<Item = LfgDungeonTemplateRowLikeCpp>,
        reward_rows: impl IntoIterator<Item = LfgDungeonRewardRowLikeCpp>,
        quest_store: &QuestStore,
    ) -> LfgLoadOutcomeLikeCpp {
        let mut store = Self::default();
        let mut report = LfgLoadReportLikeCpp::default();

        let mut db2_entries = db2_store.entries().collect::<Vec<_>>();
        db2_entries.sort_by_key(|entry| entry.id);
        for entry in db2_entries {
            report.db2_rows_seen += 1;
            match entry.type_id {
                LFG_TYPE_DUNGEON_LIKE_CPP
                | LFG_TYPE_HEROIC_LIKE_CPP
                | LFG_TYPE_RAID_LIKE_CPP
                | LFG_TYPE_RANDOM_LIKE_CPP => {}
                _ => {
                    report.skipped_type.push(entry.id);
                    continue;
                }
            }

            if entry.type_id != LFG_TYPE_RANDOM_LIKE_CPP {
                let Ok(map) = u32::try_from(entry.map_id) else {
                    report.skipped_missing_map_difficulty.push(entry.id);
                    continue;
                };
                if map_difficulty_store.get(map, entry.difficulty_id).is_none() {
                    report.skipped_missing_map_difficulty.push(entry.id);
                    continue;
                }
            }

            let Some(data) = LfgDungeonDataLikeCpp::from_db2_like_cpp(entry) else {
                report.skipped_missing_map_difficulty.push(entry.id);
                continue;
            };
            store.dungeons.insert(data.id, data);
        }
        report.loaded_dungeons = store.dungeons.len();

        for row in template_rows {
            report.template_rows_seen += 1;
            let Some(dungeon) = store.dungeons.get_mut(&row.dungeon_id) else {
                report.skipped_template_missing_dungeon.push(row.dungeon_id);
                continue;
            };
            dungeon.x = row.position_x;
            dungeon.y = row.position_y;
            dungeon.z = row.position_z;
            dungeon.o = row.orientation;
            dungeon.required_item_level = row.required_item_level;
            report.loaded_templates += 1;
        }

        for dungeon in store.dungeons.values() {
            if dungeon.type_id != LFG_TYPE_RANDOM_LIKE_CPP {
                store
                    .cached_by_random
                    .entry(u32::from(dungeon.group))
                    .or_default()
                    .insert(dungeon.id);
            }
            store
                .cached_by_random
                .entry(0)
                .or_default()
                .insert(dungeon.id);
        }

        for row in reward_rows {
            report.reward_rows_seen += 1;
            if !store.dungeons.contains_key(&row.dungeon_id) {
                report.skipped_reward_missing_dungeon.push(row.dungeon_id);
                continue;
            }
            if row.reward.first_quest_id == 0
                || quest_store.get(row.reward.first_quest_id).is_none()
            {
                report
                    .skipped_reward_missing_first_quest
                    .push((row.dungeon_id, row.reward.first_quest_id));
                continue;
            }

            let mut reward = row.reward;
            if reward.other_quest_id != 0 && quest_store.get(reward.other_quest_id).is_none() {
                report
                    .corrected_reward_missing_other_quest
                    .push((row.dungeon_id, reward.other_quest_id));
                reward.other_quest_id = 0;
            }

            store
                .rewards
                .entry(row.dungeon_id)
                .or_default()
                .push(reward);
            report.loaded_rewards += 1;
        }

        LfgLoadOutcomeLikeCpp { store, report }
    }

    pub async fn load_like_cpp(
        db: &WorldDatabase,
        db2_store: &LfgDungeonsStore,
        map_difficulty_store: &MapDifficultyStore,
        quest_store: &QuestStore,
    ) -> Result<LfgLoadOutcomeLikeCpp> {
        let mut template_rows = Vec::new();
        let mut template_result = db
            .query(&db.prepare(WorldStatements::SEL_LFG_DUNGEON_TEMPLATES))
            .await?;
        if !template_result.is_empty() {
            loop {
                template_rows.push(LfgDungeonTemplateRowLikeCpp {
                    dungeon_id: template_result.try_read::<u32>(0).unwrap_or(0),
                    position_x: template_result.try_read::<f32>(1).unwrap_or(0.0),
                    position_y: template_result.try_read::<f32>(2).unwrap_or(0.0),
                    position_z: template_result.try_read::<f32>(3).unwrap_or(0.0),
                    orientation: template_result.try_read::<f32>(4).unwrap_or(0.0),
                    required_item_level: template_result
                        .try_read::<u16>(5)
                        .or_else(|| {
                            template_result
                                .try_read::<i16>(5)
                                .and_then(|value| u16::try_from(value).ok())
                        })
                        .unwrap_or(0),
                });
                if !template_result.next_row() {
                    break;
                }
            }
        }

        let mut reward_rows = Vec::new();
        let mut reward_result = db
            .query(&db.prepare(WorldStatements::SEL_LFG_DUNGEON_REWARDS))
            .await?;
        if !reward_result.is_empty() {
            loop {
                reward_rows.push(LfgDungeonRewardRowLikeCpp {
                    dungeon_id: reward_result.try_read::<u32>(0).unwrap_or(0),
                    reward: LfgDungeonRewardLikeCpp {
                        max_level: reward_result.try_read::<u8>(1).unwrap_or(0),
                        first_quest_id: reward_result.try_read::<u32>(2).unwrap_or(0),
                        other_quest_id: reward_result.try_read::<u32>(3).unwrap_or(0),
                    },
                });
                if !reward_result.next_row() {
                    break;
                }
            }
        }

        Ok(Self::from_sources_like_cpp(
            db2_store,
            map_difficulty_store,
            template_rows,
            reward_rows,
            quest_store,
        ))
    }

    pub fn get(&self, id: u32) -> Option<&LfgDungeonDataLikeCpp> {
        self.dungeons.get(&id)
    }

    pub fn len(&self) -> usize {
        self.dungeons.len()
    }

    pub fn is_empty(&self) -> bool {
        self.dungeons.is_empty()
    }

    pub fn dungeons_by_random_like_cpp(&self, random_id: u32) -> BTreeSet<u32> {
        self.cached_by_random
            .get(&random_id)
            .cloned()
            .unwrap_or_default()
    }

    pub fn locked_dungeon_ids_like_cpp(&self) -> BTreeSet<u32> {
        self.dungeons_by_random_like_cpp(0)
    }

    pub fn random_and_active_seasonal_dungeon_entries_like_cpp(
        &self,
        level: u8,
        expansion: u8,
        is_season_active: impl Fn(u32) -> bool,
    ) -> Vec<u32> {
        let mut entries = self
            .dungeons
            .values()
            .filter(|dungeon| {
                dungeon.type_id == LFG_TYPE_RANDOM_LIKE_CPP
                    || (dungeon.seasonal && is_season_active(dungeon.id))
            })
            .filter(|dungeon| dungeon.expansion <= expansion)
            .filter(|dungeon| dungeon.min_level <= level && level <= dungeon.max_level)
            .map(LfgDungeonDataLikeCpp::entry_like_cpp)
            .collect::<Vec<_>>();
        entries.sort_unstable();
        entries
    }

    /// C++ `LFGMgr::GetRandomDungeonReward`.
    pub fn random_dungeon_reward_like_cpp(
        &self,
        dungeon_entry: u32,
        level: u8,
    ) -> Option<&LfgDungeonRewardLikeCpp> {
        let dungeon_id = dungeon_entry & 0x00FF_FFFF;
        let rewards = self.rewards.get(&dungeon_id)?;
        let mut selected = rewards.first()?;
        for reward in rewards {
            selected = reward;
            if reward.max_level >= level {
                break;
            }
        }
        Some(selected)
    }
}

impl LfgDungeonsStore {
    /// Load `LFGDungeons.db2` and overlay C++ hotfix rows from `hotfixes.lfg_dungeons`.
    pub async fn load_with_hotfixes(
        data_dir: &str,
        locale: &str,
        hotfix_db: &HotfixDatabase,
    ) -> Result<Self> {
        let mut entries = Self::load(data_dir, locale)?
            .entries()
            .cloned()
            .map(|entry| (entry.id, entry))
            .collect::<HashMap<_, _>>();

        let stmt = hotfix_db.prepare(HotfixStatements::SEL_LFG_DUNGEONS);
        let mut result = hotfix_db.query(&stmt).await?;
        if result.is_empty() {
            return Ok(Self::from_entries(entries.into_values()));
        }

        let mut hotfix_rows = 0usize;
        loop {
            let id: u32 = result.read(0);
            entries.insert(
                id,
                LfgDungeonsEntry {
                    id,
                    name: result.try_read::<String>(1).unwrap_or_default(),
                    description: result.try_read::<String>(2).unwrap_or_default(),
                    min_level: result.try_read::<u8>(3).unwrap_or(0),
                    max_level: result.try_read::<u16>(4).unwrap_or(0),
                    type_id: result.try_read::<u8>(5).unwrap_or(0),
                    subtype: result.try_read::<u8>(6).unwrap_or(0),
                    faction: result.try_read::<i8>(7).unwrap_or(0),
                    icon_texture_file_id: result.try_read::<i32>(8).unwrap_or(0),
                    rewards_bg_texture_file_id: result.try_read::<i32>(9).unwrap_or(0),
                    popup_bg_texture_file_id: result.try_read::<i32>(10).unwrap_or(0),
                    expansion_level: result.try_read::<u8>(11).unwrap_or(0),
                    map_id: result.try_read::<i16>(12).unwrap_or(0),
                    difficulty_id: result.try_read::<u8>(13).unwrap_or(0),
                    min_gear: result.try_read::<f32>(14).unwrap_or(0.0),
                    group_id: result.try_read::<u8>(15).unwrap_or(0),
                    order_index: result.try_read::<u8>(16).unwrap_or(0),
                    required_player_condition_id: result.try_read::<u32>(17).unwrap_or(0),
                    target_level: result.try_read::<u8>(18).unwrap_or(0),
                    target_level_min: result.try_read::<u8>(19).unwrap_or(0),
                    target_level_max: result.try_read::<u16>(20).unwrap_or(0),
                    random_id: result.try_read::<u16>(21).unwrap_or(0),
                    scenario_id: result.try_read::<u16>(22).unwrap_or(0),
                    final_encounter_id: result.try_read::<u16>(23).unwrap_or(0),
                    count_tank: result.try_read::<u8>(24).unwrap_or(0),
                    count_healer: result.try_read::<u8>(25).unwrap_or(0),
                    count_damage: result.try_read::<u8>(26).unwrap_or(0),
                    min_count_tank: result.try_read::<u8>(27).unwrap_or(0),
                    min_count_healer: result.try_read::<u8>(28).unwrap_or(0),
                    min_count_damage: result.try_read::<u8>(29).unwrap_or(0),
                    bonus_reputation_amount: result.try_read::<u16>(30).unwrap_or(0),
                    mentor_item_level: result.try_read::<u16>(31).unwrap_or(0),
                    mentor_char_level: result.try_read::<u8>(32).unwrap_or(0),
                    flags: [
                        result.try_read::<i32>(33).unwrap_or(0),
                        result.try_read::<i32>(34).unwrap_or(0),
                    ],
                },
            );
            hotfix_rows += 1;

            if !result.next_row() {
                break;
            }
        }

        tracing::info!("Loaded {hotfix_rows} LFGDungeons hotfix rows");
        Ok(Self::from_entries(entries.into_values()))
    }
}

pub fn lfg_dungeon_is_known_map_like_cpp(
    dungeon: &LfgDungeonDataLikeCpp,
    map_store: &MapStore,
) -> bool {
    map_store.get(dungeon.map).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::MapDifficultyEntry;

    fn lfg_entry(id: u32, type_id: u8, map_id: i16, group_id: u8) -> LfgDungeonsEntry {
        LfgDungeonsEntry {
            id,
            name: format!("LFG {id}"),
            description: String::new(),
            min_level: 10,
            max_level: 80,
            type_id,
            subtype: 0,
            faction: 0,
            icon_texture_file_id: 0,
            rewards_bg_texture_file_id: 0,
            popup_bg_texture_file_id: 0,
            expansion_level: 2,
            map_id,
            difficulty_id: 1,
            min_gear: 999.0,
            group_id,
            order_index: 0,
            required_player_condition_id: 0,
            target_level: 0,
            target_level_min: 0,
            target_level_max: 0,
            random_id: 0,
            scenario_id: 0,
            final_encounter_id: 0,
            count_tank: 0,
            count_healer: 0,
            count_damage: 0,
            min_count_tank: 0,
            min_count_healer: 0,
            min_count_damage: 0,
            bonus_reputation_amount: 0,
            mentor_item_level: 0,
            mentor_char_level: 0,
            flags: [0; 2],
        }
    }

    #[test]
    fn load_lfg_dungeons_uses_template_required_item_level_not_db2_min_gear_like_cpp() {
        let db2 = LfgDungeonsStore::from_entries([
            lfg_entry(1, LFG_TYPE_DUNGEON_LIKE_CPP, 33, 7),
            lfg_entry(2, LFG_TYPE_RANDOM_LIKE_CPP, -1, 0),
        ]);
        let map_difficulties = MapDifficultyStore::from_entries([MapDifficultyEntry {
            id: 1,
            message: String::new(),
            map_id: 33,
            difficulty_id: 1,
            lock_id: 0,
            reset_interval: 0,
            max_players: 5,
            flags: 0,
        }]);
        let quest_store = QuestStore::new();

        let outcome = LfgDungeonStoreLikeCpp::from_sources_like_cpp(
            &db2,
            &map_difficulties,
            [LfgDungeonTemplateRowLikeCpp {
                dungeon_id: 1,
                position_x: 1.0,
                position_y: 2.0,
                position_z: 3.0,
                orientation: 4.0,
                required_item_level: 168,
            }],
            [],
            &quest_store,
        );

        assert_eq!(outcome.report.loaded_dungeons, 2);
        assert_eq!(outcome.store.get(1).unwrap().required_item_level, 168);
        assert_eq!(outcome.store.get(1).unwrap().entry_like_cpp(), 16_777_217);
    }

    #[test]
    fn debug_real_lfg_dungeon_store_like_cpp() {
        let Ok(data_dir) = std::env::var("RUSTYCORE_REAL_DATA_DIR") else {
            eprintln!("Skipping real LFG debug: RUSTYCORE_REAL_DATA_DIR is not set");
            return;
        };
        let locale = std::env::var("RUSTYCORE_REAL_LOCALE").unwrap_or_else(|_| "enUS".to_string());
        let db2 = LfgDungeonsStore::load(&data_dir, &locale).expect("load LFGDungeons.db2");
        let map_difficulties =
            MapDifficultyStore::load(&data_dir, &locale).expect("load MapDifficulty.db2");
        let quest_store = QuestStore::new();
        let outcome = LfgDungeonStoreLikeCpp::from_sources_like_cpp(
            &db2,
            &map_difficulties,
            [],
            [],
            &quest_store,
        );
        eprintln!(
            "loaded={} skipped_type={:?} skipped_mapdiff={:?}",
            outcome.report.loaded_dungeons,
            outcome.report.skipped_type,
            outcome.report.skipped_missing_map_difficulty
        );
        eprintln!(
            "random_entries={:?}",
            outcome
                .store
                .random_and_active_seasonal_dungeon_entries_like_cpp(80, 2, |_| false)
        );
        eprintln!(
            "locked_entries={:?}",
            outcome
                .store
                .locked_dungeon_ids_like_cpp()
                .into_iter()
                .map(|id| outcome.store.get(id).unwrap().entry_like_cpp())
                .collect::<Vec<_>>()
        );
        let map_store = MapStore::load(&data_dir, &locale).expect("load Map.db2");
        for id in [
            0_u32, 205, 210, 211, 212, 213, 215, 217, 219, 221, 226, 241, 242, 245, 249, 252, 253,
            254, 255, 256, 257, 258, 259, 260, 261, 262, 285, 286, 287, 288, 2452, 2461, 2462,
            2471, 9259, 9260, 9261,
        ] {
            if let Some(dungeon) = outcome.store.get(id) {
                eprintln!(
                    "id={id} entry={} type={} map={} diff={} group={} min={} max={} exp={} map_known={}",
                    dungeon.entry_like_cpp(),
                    dungeon.type_id,
                    dungeon.map,
                    dungeon.difficulty,
                    dungeon.group,
                    dungeon.min_level,
                    dungeon.max_level,
                    dungeon.expansion,
                    map_store.get(dungeon.map).is_some()
                );
            } else {
                eprintln!("id={id} missing from store");
            }
        }
    }
}
