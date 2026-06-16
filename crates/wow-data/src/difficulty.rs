// Copyright (c) 2026 alseif0x
// RustyCore - WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 - https://www.gnu.org/licenses/gpl-3.0.html

//! Difficulty.db2 reader.

use std::collections::HashMap;
use std::path::Path;

use anyhow::{Context, Result};
use tracing::info;
use wow_constants::shared::DifficultyFlags;

use crate::wdc4::Wdc4Reader;

const MAP_INSTANCE_LIKE_CPP: u8 = 1;
const MAP_RAID_LIKE_CPP: u8 = 2;
const DIFFICULTY_NORMAL_LIKE_CPP: u32 = 1;
const DIFFICULTY_NORMAL_RAID_LIKE_CPP: u32 = 14;
const DIFFICULTY_10_N_LIKE_CPP: u32 = 3;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DifficultyEntry {
    pub id: u32,
    pub instance_type: u8,
    pub flags: u8,
    pub fallback_difficulty_id: u8,
    pub toggle_difficulty_id: u8,
}

/// Minimal C++ `DifficultyEntry` store for `sDifficultyStore.LookupEntry`.
pub struct DifficultyStore {
    entries: HashMap<u32, DifficultyEntry>,
}

impl DifficultyStore {
    pub fn from_ids(ids: impl IntoIterator<Item = u32>) -> Self {
        Self::from_entries(ids.into_iter().map(|id| DifficultyEntry {
            id,
            instance_type: 0,
            flags: 0,
            fallback_difficulty_id: 0,
            toggle_difficulty_id: 0,
        }))
    }

    pub fn from_entries(entries: impl IntoIterator<Item = DifficultyEntry>) -> Self {
        Self {
            entries: entries.into_iter().map(|entry| (entry.id, entry)).collect(),
        }
    }

    /// Load Difficulty.db2 from `{data_dir}/dbc/{locale}/Difficulty.db2`.
    ///
    /// C++ refs:
    /// - `DB2Stores.cpp::sDifficultyStore`
    /// - `ConditionMgr::isConditionTypeValid(CONDITION_DIFFICULTY_ID)`
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        let path = Path::new(data_dir)
            .join("dbc")
            .join(locale)
            .join("Difficulty.db2");

        let reader = Wdc4Reader::open(&path)
            .with_context(|| format!("failed to open {}", path.display()))?;
        let mut entries = HashMap::with_capacity(reader.total_count());
        for (id, idx) in reader.iter_records() {
            entries.insert(
                id,
                DifficultyEntry {
                    id,
                    // WDC4 record ids supply C++ field 0 (`ID`). Physical
                    // fields then start at `Name`, so C++ `InstanceType` and
                    // `Flags`, `FallbackDifficultyID`, and
                    // `ToggleDifficultyID` are reader fields 7, 4, and 9
                    // respectively.
                    instance_type: reader.get_field_u8(idx, 1),
                    flags: reader.get_field_u8(idx, 7),
                    fallback_difficulty_id: reader.get_field_u8(idx, 4),
                    toggle_difficulty_id: reader.get_field_u8(idx, 9),
                },
            );
        }

        info!(
            "Loaded {} difficulties from {}",
            entries.len(),
            path.display()
        );
        Ok(Self { entries })
    }

    pub fn get(&self, id: u32) -> Option<&DifficultyEntry> {
        self.entries.get(&id)
    }

    pub fn contains(&self, id: u32) -> bool {
        self.entries.contains_key(&id)
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn check_loaded_dungeon_difficulty_id_like_cpp(&self, difficulty: u32) -> u32 {
        match self.get(difficulty) {
            Some(entry)
                if entry.instance_type == MAP_INSTANCE_LIKE_CPP
                    && difficulty_can_select_like_cpp(entry) =>
            {
                difficulty
            }
            _ => DIFFICULTY_NORMAL_LIKE_CPP,
        }
    }

    pub fn check_loaded_raid_difficulty_id_like_cpp(&self, difficulty: u32) -> u32 {
        match self.get(difficulty) {
            Some(entry)
                if entry.instance_type == MAP_RAID_LIKE_CPP
                    && difficulty_can_select_like_cpp(entry)
                    && !difficulty_is_legacy_like_cpp(entry) =>
            {
                difficulty
            }
            _ => DIFFICULTY_NORMAL_RAID_LIKE_CPP,
        }
    }

    pub fn check_loaded_legacy_raid_difficulty_id_like_cpp(&self, difficulty: u32) -> u32 {
        match self.get(difficulty) {
            Some(entry)
                if entry.instance_type == MAP_RAID_LIKE_CPP
                    && difficulty_can_select_like_cpp(entry)
                    && difficulty_is_legacy_like_cpp(entry) =>
            {
                difficulty
            }
            _ => DIFFICULTY_10_N_LIKE_CPP,
        }
    }
}

fn difficulty_can_select_like_cpp(entry: &DifficultyEntry) -> bool {
    DifficultyFlags::from_bits_truncate(entry.flags).contains(DifficultyFlags::CAN_SELECT)
}

fn difficulty_is_legacy_like_cpp(entry: &DifficultyEntry) -> bool {
    DifficultyFlags::from_bits_truncate(entry.flags).contains(DifficultyFlags::LEGACY)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn difficulty_store_indexes_record_ids_like_cpp_store() {
        let store = DifficultyStore::from_ids([0, 1, 23]);

        assert!(store.contains(1));
        assert!(!store.contains(2));
        assert_eq!(store.len(), 3);
    }

    #[test]
    fn check_loaded_dungeon_difficulty_matches_player_cpp() {
        let store = DifficultyStore::from_entries([
            DifficultyEntry {
                id: 2,
                instance_type: MAP_INSTANCE_LIKE_CPP,
                flags: DifficultyFlags::CAN_SELECT.bits(),
                fallback_difficulty_id: 0,
                toggle_difficulty_id: 0,
            },
            DifficultyEntry {
                id: 19,
                instance_type: MAP_INSTANCE_LIKE_CPP,
                flags: 0,
                fallback_difficulty_id: 0,
                toggle_difficulty_id: 0,
            },
            DifficultyEntry {
                id: 15,
                instance_type: MAP_RAID_LIKE_CPP,
                flags: DifficultyFlags::CAN_SELECT.bits(),
                fallback_difficulty_id: 0,
                toggle_difficulty_id: 0,
            },
        ]);

        assert_eq!(store.check_loaded_dungeon_difficulty_id_like_cpp(2), 2);
        assert_eq!(
            store.check_loaded_dungeon_difficulty_id_like_cpp(999),
            DIFFICULTY_NORMAL_LIKE_CPP
        );
        assert_eq!(
            store.check_loaded_dungeon_difficulty_id_like_cpp(19),
            DIFFICULTY_NORMAL_LIKE_CPP
        );
        assert_eq!(
            store.check_loaded_dungeon_difficulty_id_like_cpp(15),
            DIFFICULTY_NORMAL_LIKE_CPP
        );
    }

    #[test]
    fn check_loaded_raid_difficulty_matches_player_cpp() {
        let store = DifficultyStore::from_entries([
            DifficultyEntry {
                id: 15,
                instance_type: MAP_RAID_LIKE_CPP,
                flags: DifficultyFlags::CAN_SELECT.bits(),
                fallback_difficulty_id: 0,
                toggle_difficulty_id: 0,
            },
            DifficultyEntry {
                id: 3,
                instance_type: MAP_RAID_LIKE_CPP,
                flags: (DifficultyFlags::CAN_SELECT | DifficultyFlags::LEGACY).bits(),
                fallback_difficulty_id: 0,
                toggle_difficulty_id: 0,
            },
            DifficultyEntry {
                id: 2,
                instance_type: MAP_INSTANCE_LIKE_CPP,
                flags: DifficultyFlags::CAN_SELECT.bits(),
                fallback_difficulty_id: 0,
                toggle_difficulty_id: 0,
            },
        ]);

        assert_eq!(store.check_loaded_raid_difficulty_id_like_cpp(15), 15);
        assert_eq!(
            store.check_loaded_raid_difficulty_id_like_cpp(999),
            DIFFICULTY_NORMAL_RAID_LIKE_CPP
        );
        assert_eq!(
            store.check_loaded_raid_difficulty_id_like_cpp(3),
            DIFFICULTY_NORMAL_RAID_LIKE_CPP
        );
        assert_eq!(
            store.check_loaded_raid_difficulty_id_like_cpp(2),
            DIFFICULTY_NORMAL_RAID_LIKE_CPP
        );
    }

    #[test]
    fn check_loaded_legacy_raid_difficulty_matches_player_cpp() {
        let store = DifficultyStore::from_entries([
            DifficultyEntry {
                id: 3,
                instance_type: MAP_RAID_LIKE_CPP,
                flags: (DifficultyFlags::CAN_SELECT | DifficultyFlags::LEGACY).bits(),
                fallback_difficulty_id: 0,
                toggle_difficulty_id: 0,
            },
            DifficultyEntry {
                id: 15,
                instance_type: MAP_RAID_LIKE_CPP,
                flags: DifficultyFlags::CAN_SELECT.bits(),
                fallback_difficulty_id: 0,
                toggle_difficulty_id: 0,
            },
            DifficultyEntry {
                id: 2,
                instance_type: MAP_INSTANCE_LIKE_CPP,
                flags: DifficultyFlags::CAN_SELECT.bits(),
                fallback_difficulty_id: 0,
                toggle_difficulty_id: 0,
            },
        ]);

        assert_eq!(store.check_loaded_legacy_raid_difficulty_id_like_cpp(3), 3);
        assert_eq!(
            store.check_loaded_legacy_raid_difficulty_id_like_cpp(999),
            DIFFICULTY_10_N_LIKE_CPP
        );
        assert_eq!(
            store.check_loaded_legacy_raid_difficulty_id_like_cpp(15),
            DIFFICULTY_10_N_LIKE_CPP
        );
        assert_eq!(
            store.check_loaded_legacy_raid_difficulty_id_like_cpp(2),
            DIFFICULTY_10_N_LIKE_CPP
        );
    }
}
