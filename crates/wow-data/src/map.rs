// Copyright (c) 2026 alseif0x
// RustyCore - WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 - https://www.gnu.org/licenses/gpl-3.0.html

//! Map.db2 and MapDifficulty.db2 readers.

use std::collections::{BTreeMap, HashMap};
use std::path::Path;

use anyhow::{Context, Result};
use tracing::info;
use wow_constants::shared::DifficultyFlags;

use crate::{DifficultyStore, PlayerConditionEntry, PlayerConditionStore, wdc4::Wdc4Reader};

pub const MAP_FLAG_FLEXIBLE_RAID_LOCKING: u32 = 0x0000_8000;
pub const MAP_FLAG_GARRISON: u32 = 0x0400_0000;
pub const MAP_FLAG2_ACTIVATES_PVP_ITEM_LEVELS_LIKE_CPP: u32 = 0x0000_0040;
pub const MAP_FLAG2_IGNORE_INSTANCE_FARM_LIMIT: u32 = 0x0000_0080;
pub const MAP_DIFFICULTY_FLAG_USE_LOOT_BASED_LOCK: u8 = 0x02;

pub const MAP_COMMON: i8 = 0;
pub const MAP_INSTANCE: i8 = 1;
pub const MAP_RAID: i8 = 2;
pub const MAP_BATTLEGROUND: i8 = 3;
pub const MAP_ARENA: i8 = 4;
pub const MAP_SCENARIO: i8 = 5;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MapEntry {
    pub id: u32,
    pub instance_type: i8,
    pub expansion_id: u8,
    pub parent_map_id: i16,
    pub cosmetic_parent_map_id: i16,
    pub flags1: u32,
    pub flags2: u32,
}

impl MapEntry {
    pub const fn is_flex_locking(self) -> bool {
        self.flags1 & MAP_FLAG_FLEXIBLE_RAID_LOCKING != 0
    }

    pub const fn is_garrison(self) -> bool {
        self.flags1 & MAP_FLAG_GARRISON != 0
    }

    pub const fn ignores_instance_farm_limit_like_cpp(self) -> bool {
        self.flags2 & MAP_FLAG2_IGNORE_INSTANCE_FARM_LIMIT != 0
    }

    pub const fn activates_pvp_item_levels_like_cpp(self) -> bool {
        self.flags2 & MAP_FLAG2_ACTIVATES_PVP_ITEM_LEVELS_LIKE_CPP != 0
    }

    pub const fn is_dungeon(self) -> bool {
        matches!(self.instance_type, MAP_INSTANCE | MAP_RAID | MAP_SCENARIO) && !self.is_garrison()
    }

    pub const fn expansion_like_cpp(self) -> u8 {
        self.expansion_id
    }

    pub const fn is_non_raid_dungeon_like_cpp(self) -> bool {
        self.instance_type == MAP_INSTANCE
    }

    pub const fn is_battleground_or_arena(self) -> bool {
        matches!(self.instance_type, MAP_BATTLEGROUND | MAP_ARENA)
    }

    pub const fn is_world_map(self) -> bool {
        self.instance_type == MAP_COMMON
    }

    pub const fn is_split_by_faction(self) -> bool {
        matches!(self.id, 609 | 1265 | 1481 | 2175 | 2570)
    }
}

pub struct MapStore {
    entries: HashMap<u32, MapEntry>,
}

impl MapStore {
    pub fn from_entries(entries: impl IntoIterator<Item = MapEntry>) -> Self {
        Self {
            entries: entries.into_iter().map(|entry| (entry.id, entry)).collect(),
        }
    }

    /// Load Map.db2 from `{data_dir}/dbc/{locale}/Map.db2`.
    ///
    /// C++ refs:
    /// - `DB2Structure.h::MapEntry`
    /// - `DB2LoadInfo.h::MapLoadInfo`
    /// - `DB2Stores.cpp::sMapStore`
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        let path = Path::new(data_dir).join("dbc").join(locale).join("Map.db2");
        let reader = Wdc4Reader::open(&path)
            .with_context(|| format!("failed to open {}", path.display()))?;

        let mut entries = HashMap::with_capacity(reader.total_count());
        for (id, idx) in reader.iter_records() {
            let entry = MapEntry {
                id,
                // WDC4 record ids supply C++ field 0 (`ID`) and this reader
                // exposes `Flags[3]` as one array field, so C++ field 8 -> 7,
                // C++ field 9 -> 8, C++ fields 13..14 -> fields 12..13,
                // and C++ fields 22..24 -> field 21 with array elements 0..2.
                instance_type: reader.get_field_i8(idx, 7),
                expansion_id: reader.get_field_u8(idx, 8),
                parent_map_id: reader.get_field_i16(idx, 12),
                cosmetic_parent_map_id: reader.get_field_i16(idx, 13),
                flags1: reader.get_field_u32(idx, 21),
                flags2: reader.get_array_element(idx, 21, 1, 32),
            };
            entries.insert(id, entry);
        }

        info!("Loaded {} maps from {}", entries.len(), path.display());
        Ok(Self { entries })
    }

    pub fn get(&self, id: u32) -> Option<&MapEntry> {
        self.entries.get(&id)
    }

    pub fn entries(&self) -> impl Iterator<Item = &MapEntry> {
        self.entries.values()
    }

    /// Resolve the root terrain map the same way C++ `TerrainMgr::LoadTerrain`
    /// does before loading grid/vmap files.
    pub fn terrain_root_map_id_like_cpp(&self, map_id: u32) -> Option<u32> {
        let mut current_map_id = map_id;
        let mut entry = self.get(current_map_id)?;

        while entry.parent_map_id != -1 || entry.cosmetic_parent_map_id != -1 {
            let parent_map_id = if entry.parent_map_id != -1 {
                entry.parent_map_id
            } else {
                entry.cosmetic_parent_map_id
            };

            let Ok(parent_map_id) = u32::try_from(parent_map_id) else {
                break;
            };
            let Some(parent_entry) = self.get(parent_map_id) else {
                break;
            };

            current_map_id = parent_map_id;
            entry = parent_entry;
        }

        Some(current_map_id)
    }

    /// Build C++ `World::SetInitialWorldSettings` mapData:
    /// every map id is present and direct child maps are attached to
    /// `ParentMapID`, or `CosmeticParentMapID` when no parent exists.
    pub fn parent_child_map_data_like_cpp(&self) -> Vec<(u32, Vec<u32>)> {
        let mut map_data = BTreeMap::<u32, Vec<u32>>::new();
        for entry in self.entries.values() {
            map_data.entry(entry.id).or_default();
            if entry.parent_map_id != -1 {
                assert!(
                    entry.cosmetic_parent_map_id == -1
                        || entry.cosmetic_parent_map_id == entry.parent_map_id,
                    "inconsistent parent map data for map {} (ParentMapID = {}, CosmeticParentMapID = {})",
                    entry.id,
                    entry.parent_map_id,
                    entry.cosmetic_parent_map_id
                );
                if let Ok(parent_map_id) = u32::try_from(entry.parent_map_id) {
                    map_data.entry(parent_map_id).or_default().push(entry.id);
                }
            } else if entry.cosmetic_parent_map_id != -1 {
                if let Ok(parent_map_id) = u32::try_from(entry.cosmetic_parent_map_id) {
                    map_data.entry(parent_map_id).or_default().push(entry.id);
                }
            }
        }

        map_data.into_iter().collect()
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MapDifficultyEntry {
    pub id: u32,
    pub message: String,
    pub map_id: u32,
    pub difficulty_id: u8,
    pub lock_id: u8,
    pub reset_interval: u8,
    pub max_players: u32,
    pub flags: u8,
}

impl MapDifficultyEntry {
    pub const fn is_using_encounter_locks(&self) -> bool {
        self.flags & MAP_DIFFICULTY_FLAG_USE_LOOT_BASED_LOCK != 0
    }
}

pub struct MapDifficultyStore {
    by_id: HashMap<u32, MapDifficultyEntry>,
    by_map_difficulty: HashMap<(u32, u8), u32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MapDifficultyXConditionEntry {
    pub id: u32,
    pub failure_description: String,
    pub player_condition_id: u32,
    pub order_index: i32,
    pub map_difficulty_id: u32,
}

pub struct MapDifficultyXConditionStore {
    by_id: HashMap<u32, MapDifficultyXConditionEntry>,
    by_map_difficulty: HashMap<u32, Vec<u32>>,
}

impl MapDifficultyXConditionStore {
    pub fn from_entries(entries: impl IntoIterator<Item = MapDifficultyXConditionEntry>) -> Self {
        let mut entries: Vec<_> = entries.into_iter().collect();
        entries.sort_by_key(|entry| entry.order_index);

        let mut by_id = HashMap::new();
        let mut by_map_difficulty = HashMap::<u32, Vec<u32>>::new();
        for entry in entries {
            by_map_difficulty
                .entry(entry.map_difficulty_id)
                .or_default()
                .push(entry.id);
            by_id.insert(entry.id, entry);
        }

        Self {
            by_id,
            by_map_difficulty,
        }
    }

    /// Load MapDifficultyXCondition.db2 from `{data_dir}/dbc/{locale}/MapDifficultyXCondition.db2`.
    ///
    /// C++ refs:
    /// - `DB2Structure.h::MapDifficultyXConditionEntry`
    /// - `DB2LoadInfo.h::MapDifficultyXConditionLoadInfo`
    /// - `DB2Stores.cpp` post-load sort and `_mapDifficultyConditions` build.
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        let path = Path::new(data_dir)
            .join("dbc")
            .join(locale)
            .join("MapDifficultyXCondition.db2");
        let reader = Wdc4Reader::open(&path)
            .with_context(|| format!("failed to open {}", path.display()))?;

        let mut entries = Vec::with_capacity(reader.total_count());
        for (id, idx) in reader.iter_records() {
            entries.push(MapDifficultyXConditionEntry {
                id,
                failure_description: reader.get_field_string(idx, 0),
                player_condition_id: reader.get_field_u32(idx, 1),
                order_index: reader.get_field_i32(idx, 2),
                map_difficulty_id: reader.get_relationship_id(idx).unwrap_or(0),
            });
        }

        let store = Self::from_entries(entries);
        info!(
            "Loaded {} map difficulty conditions from {}",
            store.len(),
            path.display()
        );
        Ok(store)
    }

    pub fn get(&self, id: u32) -> Option<&MapDifficultyXConditionEntry> {
        self.by_id.get(&id)
    }

    pub fn conditions_for_map_difficulty(
        &self,
        map_difficulty_id: u32,
    ) -> impl Iterator<Item = &MapDifficultyXConditionEntry> {
        self.by_map_difficulty
            .get(&map_difficulty_id)
            .into_iter()
            .flat_map(|ids| ids.iter())
            .filter_map(|id| self.by_id.get(id))
    }

    pub fn failed_condition_like_cpp<'a>(
        &'a self,
        map_difficulty_id: u32,
        player_conditions: &'a PlayerConditionStore,
        mut meets: impl FnMut(&'a PlayerConditionEntry) -> bool,
    ) -> Option<u32> {
        for entry in self.conditions_for_map_difficulty(map_difficulty_id) {
            let Some(player_condition) = player_conditions.get(entry.player_condition_id) else {
                continue;
            };

            if !meets(player_condition) {
                return Some(entry.id);
            }
        }

        None
    }

    pub fn len(&self) -> usize {
        self.by_id.len()
    }

    pub fn is_empty(&self) -> bool {
        self.by_id.is_empty()
    }
}

impl MapDifficultyStore {
    pub fn from_entries(entries: impl IntoIterator<Item = MapDifficultyEntry>) -> Self {
        let mut by_id = HashMap::new();
        let mut by_map_difficulty = HashMap::new();
        for entry in entries {
            by_map_difficulty.insert((entry.map_id, entry.difficulty_id), entry.id);
            by_id.insert(entry.id, entry);
        }

        Self {
            by_id,
            by_map_difficulty,
        }
    }

    /// Load MapDifficulty.db2 from `{data_dir}/dbc/{locale}/MapDifficulty.db2`.
    ///
    /// C++ refs:
    /// - `DB2Structure.h::MapDifficultyEntry`
    /// - `DB2LoadInfo.h::MapDifficultyLoadInfo`
    /// - `DB2Manager::GetMapDifficultyData`
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        let path = Path::new(data_dir)
            .join("dbc")
            .join(locale)
            .join("MapDifficulty.db2");
        let reader = Wdc4Reader::open(&path)
            .with_context(|| format!("failed to open {}", path.display()))?;

        let mut entries = Vec::with_capacity(reader.total_count());
        for (id, idx) in reader.iter_records() {
            entries.push(MapDifficultyEntry {
                id,
                message: reader.get_field_string(idx, 0),
                // WDC4 record ids supply C++ field 0 (`ID`) and this reader
                // exposes the numeric payload in physical order:
                // ContentTuning, ItemContextPicker, ItemContext,
                // DifficultyID, LockID, ResetInterval, MaxPlayers, Flags, MapID.
                difficulty_id: reader.get_field_u8(idx, 3),
                lock_id: reader.get_field_u8(idx, 4),
                reset_interval: reader.get_field_u8(idx, 5),
                max_players: reader.get_field_u32(idx, 6),
                flags: reader.get_field_u8(idx, 7),
                // In 3.4.3 WDC4 data, C++ field `MapID` is sometimes exposed
                // as the physical payload field and sometimes only as the
                // relationship id. Prefer the non-zero physical field, matching
                // older rows, and fall back to the WDC4 relationship for rows
                // such as Northrend normal (map 571, difficulty 0).
                map_id: {
                    let physical_map_id = reader.get_field_u32(idx, 8);
                    if physical_map_id != 0 {
                        physical_map_id
                    } else {
                        reader.get_relationship_id(idx).unwrap_or(0)
                    }
                },
            });
        }

        let store = Self::from_entries(entries);
        info!(
            "Loaded {} map difficulties from {}",
            store.len(),
            path.display()
        );
        Ok(store)
    }

    pub fn get(&self, map_id: u32, difficulty_id: u8) -> Option<&MapDifficultyEntry> {
        self.by_map_difficulty
            .get(&(map_id, difficulty_id))
            .and_then(|id| self.by_id.get(id))
    }

    /// C++ `DB2Manager::GetDefaultMapDifficulty`.
    ///
    /// Trinity first returns the map difficulty whose `DifficultyEntry` has
    /// `DIFFICULTY_FLAG_DEFAULT`; if no default row is marked, it falls back
    /// to the first stored row for that map.
    pub fn default_for_map_like_cpp(
        &self,
        map_id: u32,
        difficulty_store: &DifficultyStore,
    ) -> Option<&MapDifficultyEntry> {
        let mut fallback = None;
        for ((entry_map_id, difficulty_id), entry_id) in &self.by_map_difficulty {
            if *entry_map_id != map_id {
                continue;
            }

            let Some(entry) = self.by_id.get(entry_id) else {
                continue;
            };
            fallback = fallback.or(Some(entry));

            let Some(difficulty) = difficulty_store.get(u32::from(*difficulty_id)) else {
                continue;
            };
            if DifficultyFlags::from_bits_truncate(difficulty.flags)
                .contains(DifficultyFlags::DEFAULT)
            {
                return Some(entry);
            }
        }

        fallback
    }

    /// C++ `DB2Manager::GetDownscaledMapDifficultyData`.
    ///
    /// Returns the selected `MapDifficultyEntry` plus the effective difficulty
    /// id after following `DifficultyEntry::FallbackDifficultyID`. If the
    /// requested difficulty or fallback chain is missing, C++ falls back to
    /// `GetDefaultMapDifficulty` and mutates the caller's difficulty id to the
    /// default row.
    pub fn downscaled_for_map_like_cpp(
        &self,
        map_id: u32,
        difficulty_id: u8,
        difficulty_store: &DifficultyStore,
    ) -> Option<(&MapDifficultyEntry, u8)> {
        let Some(mut difficulty) = difficulty_store.get(u32::from(difficulty_id)) else {
            let default = self.default_for_map_like_cpp(map_id, difficulty_store)?;
            return Some((default, default.difficulty_id));
        };

        let mut effective_difficulty_id = difficulty_id;
        loop {
            if let Some(map_difficulty) = self.get(map_id, effective_difficulty_id) {
                return Some((map_difficulty, effective_difficulty_id));
            }

            effective_difficulty_id = difficulty.fallback_difficulty_id;
            let Some(next_difficulty) = difficulty_store.get(u32::from(effective_difficulty_id))
            else {
                let default = self.default_for_map_like_cpp(map_id, difficulty_store)?;
                return Some((default, default.difficulty_id));
            };
            difficulty = next_difficulty;
        }
    }

    pub fn len(&self) -> usize {
        self.by_id.len()
    }

    pub fn is_empty(&self) -> bool {
        self.by_id.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn map_store_flex_locking_flag_matches_cpp() {
        let store = MapStore::from_entries([MapEntry {
            id: 631,
            instance_type: 2,
            expansion_id: 0,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: MAP_FLAG_FLEXIBLE_RAID_LOCKING,
            flags2: 0,
        }]);

        assert!(store.get(631).unwrap().is_flex_locking());
        assert!(store.get(1).is_none());
    }

    #[test]
    fn map_store_ignore_instance_farm_limit_flag_matches_cpp() {
        let entry = MapEntry {
            id: 33,
            instance_type: MAP_INSTANCE,
            expansion_id: 0,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: MAP_FLAG2_IGNORE_INSTANCE_FARM_LIMIT,
        };

        assert!(entry.ignores_instance_farm_limit_like_cpp());
    }

    #[test]
    fn map_entry_classification_matches_cpp_helpers() {
        let world = MapEntry {
            id: 0,
            instance_type: MAP_COMMON,
            expansion_id: 0,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        };
        let dungeon = MapEntry {
            id: 33,
            instance_type: MAP_INSTANCE,
            expansion_id: 0,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        };
        let raid = MapEntry {
            id: 631,
            instance_type: MAP_RAID,
            expansion_id: 2,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        };
        let battleground = MapEntry {
            id: 489,
            instance_type: MAP_BATTLEGROUND,
            expansion_id: 0,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        };
        let arena = MapEntry {
            id: 562,
            instance_type: MAP_ARENA,
            expansion_id: 0,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        };
        let garrison = MapEntry {
            id: 1152,
            instance_type: MAP_INSTANCE,
            expansion_id: 0,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: MAP_FLAG_GARRISON,
            flags2: 0,
        };
        let split = MapEntry {
            id: 609,
            instance_type: MAP_COMMON,
            expansion_id: 0,
            parent_map_id: 571,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        };

        assert!(world.is_world_map());
        assert!(dungeon.is_dungeon());
        assert!(raid.is_dungeon());
        assert_eq!(raid.expansion_like_cpp(), 2);
        assert!(battleground.is_battleground_or_arena());
        assert!(arena.is_battleground_or_arena());
        assert!(garrison.is_garrison());
        assert!(!garrison.is_dungeon());
        assert!(split.is_split_by_faction());

        let pvp_item_level_map = MapEntry {
            id: 30_001,
            instance_type: MAP_COMMON,
            expansion_id: 0,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: MAP_FLAG2_ACTIVATES_PVP_ITEM_LEVELS_LIKE_CPP,
        };
        assert!(
            pvp_item_level_map.activates_pvp_item_levels_like_cpp(),
            "C++ Player::UpdateItemLevelAreaBasedScaling checks MapEntry::Flags[1] & 0x40"
        );
    }

    #[test]
    fn map_store_parent_fields_match_cpp_load_info() {
        let store = MapStore::from_entries([
            MapEntry {
                id: 609,
                instance_type: 0,
                expansion_id: 0,
                parent_map_id: 571,
                cosmetic_parent_map_id: -1,
                flags1: 0,
                flags2: 0,
            },
            MapEntry {
                id: 111,
                instance_type: 0,
                expansion_id: 0,
                parent_map_id: -1,
                cosmetic_parent_map_id: 1,
                flags1: 0,
                flags2: 0,
            },
        ]);

        let child = store.get(609).unwrap();
        assert_eq!(child.parent_map_id, 571);
        assert_eq!(child.cosmetic_parent_map_id, -1);
        let cosmetic = store.get(111).unwrap();
        assert_eq!(cosmetic.parent_map_id, -1);
        assert_eq!(cosmetic.cosmetic_parent_map_id, 1);
    }

    #[test]
    fn terrain_root_map_id_follows_parent_chain_like_cpp() {
        let store = MapStore::from_entries([
            MapEntry {
                id: 1,
                instance_type: 0,
                expansion_id: 0,
                parent_map_id: -1,
                cosmetic_parent_map_id: -1,
                flags1: 0,
                flags2: 0,
            },
            MapEntry {
                id: 571,
                instance_type: 0,
                expansion_id: 0,
                parent_map_id: -1,
                cosmetic_parent_map_id: 1,
                flags1: 0,
                flags2: 0,
            },
            MapEntry {
                id: 609,
                instance_type: 0,
                expansion_id: 0,
                parent_map_id: 571,
                cosmetic_parent_map_id: -1,
                flags1: 0,
                flags2: 0,
            },
        ]);

        assert_eq!(store.terrain_root_map_id_like_cpp(609), Some(1));
        assert_eq!(store.terrain_root_map_id_like_cpp(571), Some(1));
        assert_eq!(store.terrain_root_map_id_like_cpp(1), Some(1));
    }

    #[test]
    fn terrain_root_map_id_prefers_parent_over_cosmetic_like_cpp() {
        let store = MapStore::from_entries([
            MapEntry {
                id: 1,
                instance_type: 0,
                expansion_id: 0,
                parent_map_id: -1,
                cosmetic_parent_map_id: -1,
                flags1: 0,
                flags2: 0,
            },
            MapEntry {
                id: 571,
                instance_type: 0,
                expansion_id: 0,
                parent_map_id: -1,
                cosmetic_parent_map_id: -1,
                flags1: 0,
                flags2: 0,
            },
            MapEntry {
                id: 609,
                instance_type: 0,
                expansion_id: 0,
                parent_map_id: 571,
                cosmetic_parent_map_id: 1,
                flags1: 0,
                flags2: 0,
            },
        ]);

        assert_eq!(store.terrain_root_map_id_like_cpp(609), Some(571));
    }

    #[test]
    fn terrain_root_map_id_handles_missing_entries_like_cpp() {
        let store = MapStore::from_entries([MapEntry {
            id: 609,
            instance_type: 0,
            expansion_id: 0,
            parent_map_id: 571,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        }]);

        assert_eq!(store.terrain_root_map_id_like_cpp(609), Some(609));
        assert_eq!(store.terrain_root_map_id_like_cpp(571), None);
    }

    #[test]
    fn parent_child_map_data_matches_cpp_world_initialization() {
        let store = MapStore::from_entries([
            MapEntry {
                id: 1,
                instance_type: 0,
                expansion_id: 0,
                parent_map_id: -1,
                cosmetic_parent_map_id: -1,
                flags1: 0,
                flags2: 0,
            },
            MapEntry {
                id: 571,
                instance_type: 0,
                expansion_id: 0,
                parent_map_id: -1,
                cosmetic_parent_map_id: 1,
                flags1: 0,
                flags2: 0,
            },
            MapEntry {
                id: 609,
                instance_type: 0,
                expansion_id: 0,
                parent_map_id: 571,
                cosmetic_parent_map_id: -1,
                flags1: 0,
                flags2: 0,
            },
        ]);

        let map_data = store.parent_child_map_data_like_cpp();
        assert_eq!(
            map_data,
            vec![(1, vec![571]), (571, vec![609]), (609, Vec::new())]
        );
    }

    #[test]
    fn parent_child_map_data_creates_missing_parent_bucket_like_cpp() {
        let store = MapStore::from_entries([MapEntry {
            id: 609,
            instance_type: 0,
            expansion_id: 0,
            parent_map_id: 571,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        }]);

        let map_data = store.parent_child_map_data_like_cpp();
        assert_eq!(map_data, vec![(571, vec![609]), (609, Vec::new())]);
    }

    #[test]
    #[should_panic(expected = "inconsistent parent map data")]
    fn parent_child_map_data_rejects_inconsistent_parent_like_cpp_assert() {
        let store = MapStore::from_entries([MapEntry {
            id: 609,
            instance_type: 0,
            expansion_id: 0,
            parent_map_id: 571,
            cosmetic_parent_map_id: 1,
            flags1: 0,
            flags2: 0,
        }]);

        let _ = store.parent_child_map_data_like_cpp();
    }

    #[test]
    fn map_difficulty_store_indexes_by_map_and_difficulty_like_cpp() {
        let store = MapDifficultyStore::from_entries([MapDifficultyEntry {
            id: 900,
            message: String::new(),
            map_id: 631,
            difficulty_id: 4,
            lock_id: 7,
            reset_interval: 2,
            max_players: 0,
            flags: MAP_DIFFICULTY_FLAG_USE_LOOT_BASED_LOCK,
        }]);

        let entry = store.get(631, 4).unwrap();
        assert_eq!(entry.lock_id, 7);
        assert!(entry.is_using_encounter_locks());
        assert!(store.get(631, 3).is_none());
    }

    #[test]
    fn default_for_map_prefers_default_difficulty_flag_like_cpp() {
        let difficulties = DifficultyStore::from_entries([
            crate::DifficultyEntry {
                id: 3,
                instance_type: 2,
                flags: DifficultyFlags::CAN_SELECT.bits(),
                fallback_difficulty_id: 0,
                toggle_difficulty_id: 0,
            },
            crate::DifficultyEntry {
                id: 15,
                instance_type: 2,
                flags: (DifficultyFlags::CAN_SELECT | DifficultyFlags::DEFAULT).bits(),
                fallback_difficulty_id: 0,
                toggle_difficulty_id: 0,
            },
        ]);
        let store = MapDifficultyStore::from_entries([
            MapDifficultyEntry {
                id: 900,
                message: String::new(),
                map_id: 631,
                difficulty_id: 3,
                lock_id: 0,
                reset_interval: 0,
                max_players: 0,
                flags: 0,
            },
            MapDifficultyEntry {
                id: 901,
                message: String::new(),
                map_id: 631,
                difficulty_id: 15,
                lock_id: 0,
                reset_interval: 0,
                max_players: 0,
                flags: 0,
            },
        ]);

        let entry = store.default_for_map_like_cpp(631, &difficulties).unwrap();

        assert_eq!(entry.difficulty_id, 15);
    }

    #[test]
    fn default_for_map_falls_back_to_any_map_difficulty_like_cpp() {
        let difficulties = DifficultyStore::from_entries([crate::DifficultyEntry {
            id: 3,
            instance_type: 2,
            flags: DifficultyFlags::CAN_SELECT.bits(),
            fallback_difficulty_id: 0,
            toggle_difficulty_id: 0,
        }]);
        let store = MapDifficultyStore::from_entries([MapDifficultyEntry {
            id: 900,
            message: String::new(),
            map_id: 631,
            difficulty_id: 3,
            lock_id: 0,
            reset_interval: 0,
            max_players: 0,
            flags: 0,
        }]);

        let entry = store.default_for_map_like_cpp(631, &difficulties).unwrap();

        assert_eq!(entry.difficulty_id, 3);
        assert!(store.default_for_map_like_cpp(632, &difficulties).is_none());
    }

    #[test]
    fn downscaled_for_map_uses_exact_map_difficulty_like_cpp() {
        let difficulties = DifficultyStore::from_entries([crate::DifficultyEntry {
            id: 2,
            instance_type: 1,
            flags: 0,
            fallback_difficulty_id: 1,
            toggle_difficulty_id: 0,
        }]);
        let store = MapDifficultyStore::from_entries([MapDifficultyEntry {
            id: 900,
            message: String::new(),
            map_id: 33,
            difficulty_id: 2,
            lock_id: 8,
            reset_interval: 1,
            max_players: 0,
            flags: 0,
        }]);

        let (entry, effective_difficulty) = store
            .downscaled_for_map_like_cpp(33, 2, &difficulties)
            .unwrap();

        assert_eq!(entry.lock_id, 8);
        assert_eq!(effective_difficulty, 2);
    }

    #[test]
    fn downscaled_for_map_follows_fallback_difficulty_like_cpp() {
        let difficulties = DifficultyStore::from_entries([
            crate::DifficultyEntry {
                id: 5,
                instance_type: 1,
                flags: 0,
                fallback_difficulty_id: 2,
                toggle_difficulty_id: 0,
            },
            crate::DifficultyEntry {
                id: 2,
                instance_type: 1,
                flags: 0,
                fallback_difficulty_id: 1,
                toggle_difficulty_id: 0,
            },
        ]);
        let store = MapDifficultyStore::from_entries([MapDifficultyEntry {
            id: 900,
            message: String::new(),
            map_id: 33,
            difficulty_id: 2,
            lock_id: 8,
            reset_interval: 1,
            max_players: 0,
            flags: 0,
        }]);

        let (entry, effective_difficulty) = store
            .downscaled_for_map_like_cpp(33, 5, &difficulties)
            .unwrap();

        assert_eq!(entry.difficulty_id, 2);
        assert_eq!(entry.lock_id, 8);
        assert_eq!(effective_difficulty, 2);
    }

    #[test]
    fn downscaled_for_map_falls_back_to_default_when_chain_breaks_like_cpp() {
        let difficulties = DifficultyStore::from_entries([
            crate::DifficultyEntry {
                id: 5,
                instance_type: 1,
                flags: 0,
                fallback_difficulty_id: 99,
                toggle_difficulty_id: 0,
            },
            crate::DifficultyEntry {
                id: 1,
                instance_type: 1,
                flags: DifficultyFlags::DEFAULT.bits(),
                fallback_difficulty_id: 0,
                toggle_difficulty_id: 0,
            },
        ]);
        let store = MapDifficultyStore::from_entries([MapDifficultyEntry {
            id: 900,
            message: String::new(),
            map_id: 33,
            difficulty_id: 1,
            lock_id: 7,
            reset_interval: 0,
            max_players: 0,
            flags: 0,
        }]);

        let (entry, effective_difficulty) = store
            .downscaled_for_map_like_cpp(33, 5, &difficulties)
            .unwrap();

        assert_eq!(entry.difficulty_id, 1);
        assert_eq!(entry.lock_id, 7);
        assert_eq!(effective_difficulty, 1);
    }

    #[test]
    fn map_difficulty_x_conditions_are_grouped_in_cpp_order() {
        let store = MapDifficultyXConditionStore::from_entries([
            MapDifficultyXConditionEntry {
                id: 10,
                failure_description: "late".to_string(),
                player_condition_id: 100,
                order_index: 20,
                map_difficulty_id: 7,
            },
            MapDifficultyXConditionEntry {
                id: 11,
                failure_description: "early".to_string(),
                player_condition_id: 101,
                order_index: 10,
                map_difficulty_id: 7,
            },
            MapDifficultyXConditionEntry {
                id: 12,
                failure_description: "other".to_string(),
                player_condition_id: 102,
                order_index: 1,
                map_difficulty_id: 8,
            },
        ]);

        let ids: Vec<_> = store
            .conditions_for_map_difficulty(7)
            .map(|entry| entry.id)
            .collect();
        assert_eq!(ids, vec![11, 10]);
    }

    #[test]
    fn map_difficulty_x_condition_failure_matches_cpp_first_unmet_existing_condition() {
        let store = MapDifficultyXConditionStore::from_entries([
            MapDifficultyXConditionEntry {
                id: 10,
                failure_description: String::new(),
                player_condition_id: 100,
                order_index: 10,
                map_difficulty_id: 7,
            },
            MapDifficultyXConditionEntry {
                id: 11,
                failure_description: String::new(),
                player_condition_id: 999,
                order_index: 20,
                map_difficulty_id: 7,
            },
            MapDifficultyXConditionEntry {
                id: 12,
                failure_description: String::new(),
                player_condition_id: 101,
                order_index: 30,
                map_difficulty_id: 7,
            },
        ]);
        let player_conditions = PlayerConditionStore::from_entries([
            PlayerConditionEntry {
                id: 100,
                ..Default::default()
            },
            PlayerConditionEntry {
                id: 101,
                ..Default::default()
            },
        ]);

        assert_eq!(
            store.failed_condition_like_cpp(7, &player_conditions, |condition| condition.id == 100),
            Some(12)
        );
    }

    #[test]
    fn load_map_and_map_difficulty_db2_when_fixtures_exist() {
        let data_dir = "/home/server/woltk-server-core/Data";
        let locale = "esES";
        let dbc_dir = Path::new(data_dir).join("dbc").join(locale);
        if !dbc_dir.join("Map.db2").exists() || !dbc_dir.join("MapDifficulty.db2").exists() {
            eprintln!("Skipping test: Map.db2/MapDifficulty.db2 not found");
            return;
        }

        let maps = MapStore::load(data_dir, locale).expect("failed to load maps");
        let difficulties =
            MapDifficultyStore::load(data_dir, locale).expect("failed to load map difficulties");
        let difficulty_conditions = if dbc_dir.join("MapDifficultyXCondition.db2").exists() {
            Some(
                MapDifficultyXConditionStore::load(data_dir, locale)
                    .expect("failed to load map difficulty conditions"),
            )
        } else {
            None
        };

        assert!(!maps.is_empty());
        assert!(!difficulties.is_empty());
        if let Some(difficulty_conditions) = difficulty_conditions {
            assert!(
                difficulty_conditions
                    .by_id
                    .values()
                    .all(|condition| condition.map_difficulty_id != 0)
            );
        }
        assert!(maps.get(0).is_some());

        let icc = maps.get(631).expect("Icecrown Citadel map missing");
        assert_eq!(icc.instance_type, 2);

        let known_difficulty = difficulties
            .get(32, 4)
            .expect("known MapDifficulty row for map 32 difficulty 4 missing");
        assert_eq!(known_difficulty.map_id, 32);
        assert_eq!(known_difficulty.difficulty_id, 4);
        assert_eq!(known_difficulty.reset_interval, 2);

        let northrend_normal = difficulties
            .get(571, 0)
            .expect("known MapDifficulty row for Northrend map 571 difficulty 0 missing");
        assert_eq!(northrend_normal.map_id, 571);
        assert_eq!(northrend_normal.difficulty_id, 0);
    }
}
