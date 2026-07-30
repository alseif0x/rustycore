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
use wow_database::{HotfixDatabase, HotfixStatements, SqlResult};

use crate::Db2HotfixRemovalStoreLikeCpp;
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
    table_hash_like_cpp: Option<u32>,
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
            table_hash_like_cpp: None,
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
        let table_hash = reader.table_hash();
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
        Ok(Self {
            entries,
            table_hash_like_cpp: Some(table_hash),
        })
    }

    /// Load the effective C++ `sDifficultyStore` authority.
    ///
    /// C++ refs:
    /// - `DB2StorageBase::LoadFromDB`: WDC4, official SQL, then custom SQL;
    /// - `HotfixDatabase.cpp::HOTFIX_SEL_DIFFICULTY`: exact SQL field order;
    /// - `DB2Manager::LoadHotfixData`: final `(TableHash, RecordID)` removals.
    pub async fn load_effective_like_cpp(
        data_dir: &str,
        locale: &str,
        hotfix_db: &HotfixDatabase,
        removals: &Db2HotfixRemovalStoreLikeCpp,
    ) -> Result<Self> {
        const DIFFICULTY_OVERLAY_SQL: &str = concat!(
            "SELECT ID, Name, InstanceType, OrderIndex, OldEnumValue, FallbackDifficultyID, ",
            "MinPlayers, MaxPlayers, Flags, ItemContext, ToggleDifficultyID, ",
            "GroupSizeHealthCurveID, GroupSizeDmgCurveID, GroupSizeSpellPointsCurveID ",
            "FROM difficulty WHERE (`VerifiedBuild` > 0) = ?"
        );

        let mut store = Self::load(data_dir, locale)?;
        let table_hash = store
            .table_hash_like_cpp
            .context("Difficulty.db2 is missing its WDC4 table hash")?;
        let mut overlay_batches = [Vec::new(), Vec::new()];

        // `DB2StorageBase::LoadFromDB` calls `Load(false)` before
        // `Load(true)`. The loader binds `!custom`, so official
        // (`VerifiedBuild > 0`) records precede custom records.
        for (batch_index, official) in [true, false].into_iter().enumerate() {
            let mut statement = hotfix_db.prepare(HotfixStatements::base(DIFFICULTY_OVERLAY_SQL));
            statement.set_bool(0, official);
            let mut result = hotfix_db
                .query(&statement)
                .await
                .context("failed to load Difficulty.db2 SQL overlay")?;
            if result.is_empty() {
                continue;
            }

            loop {
                overlay_batches[batch_index].push(DifficultyEntry {
                    id: read_u32_like_cpp(&result, 0),
                    instance_type: read_u8_like_cpp(&result, 2),
                    fallback_difficulty_id: read_u8_like_cpp(&result, 5),
                    flags: read_u8_like_cpp(&result, 8),
                    toggle_difficulty_id: read_u8_like_cpp(&result, 10),
                });
                if !result.next_row() {
                    break;
                }
            }
        }

        let [official_overlay_entries, custom_overlay_entries] = overlay_batches;
        store.entries = compose_effective_difficulty_entries_like_cpp(
            std::mem::take(&mut store.entries).into_values(),
            official_overlay_entries,
            custom_overlay_entries,
            table_hash,
            removals,
        );
        Ok(store)
    }

    pub fn get(&self, id: u32) -> Option<&DifficultyEntry> {
        self.entries.get(&id)
    }

    pub fn fallback_difficulty_id_like_cpp(&self, id: u8) -> Option<u8> {
        self.get(u32::from(id))
            .map(|entry| entry.fallback_difficulty_id)
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

    pub fn table_hash_like_cpp(&self) -> Option<u32> {
        self.table_hash_like_cpp
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

fn compose_effective_difficulty_entries_like_cpp(
    base_entries: impl IntoIterator<Item = DifficultyEntry>,
    official_overlay_entries: impl IntoIterator<Item = DifficultyEntry>,
    custom_overlay_entries: impl IntoIterator<Item = DifficultyEntry>,
    table_hash: u32,
    removals: &Db2HotfixRemovalStoreLikeCpp,
) -> HashMap<u32, DifficultyEntry> {
    let mut effective_entries: HashMap<_, _> = base_entries
        .into_iter()
        .map(|entry| (entry.id, entry))
        .collect();

    for entry in official_overlay_entries {
        effective_entries.insert(entry.id, entry);
    }
    for entry in custom_overlay_entries {
        effective_entries.insert(entry.id, entry);
    }

    effective_entries
        .retain(|record_id, _| !removals.contains_like_cpp(table_hash, *record_id as i32));
    effective_entries
}

fn read_u32_like_cpp(result: &SqlResult, column: usize) -> u32 {
    result
        .try_read::<u32>(column)
        .or_else(|| result.try_read::<i32>(column).map(|value| value as u32))
        .or_else(|| result.try_read::<u64>(column).map(|value| value as u32))
        .or_else(|| result.try_read::<i64>(column).map(|value| value as u32))
        .unwrap_or(0)
}

fn read_u8_like_cpp(result: &SqlResult, column: usize) -> u8 {
    result
        .try_read::<u8>(column)
        .or_else(|| result.try_read::<u16>(column).map(|value| value as u8))
        .or_else(|| result.try_read::<u32>(column).map(|value| value as u8))
        .or_else(|| result.try_read::<i32>(column).map(|value| value as u8))
        .unwrap_or(0)
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

    fn entry(
        id: u32,
        instance_type: u8,
        fallback_difficulty_id: u8,
        flags: u8,
        toggle_difficulty_id: u8,
    ) -> DifficultyEntry {
        DifficultyEntry {
            id,
            instance_type,
            flags,
            fallback_difficulty_id,
            toggle_difficulty_id,
        }
    }

    #[test]
    fn difficulty_store_indexes_record_ids_like_cpp_store() {
        let store = DifficultyStore::from_ids([0, 1, 23]);

        assert!(store.contains(1));
        assert!(!store.contains(2));
        assert_eq!(store.len(), 3);
        assert_eq!(store.table_hash_like_cpp(), None);
    }

    #[test]
    fn official_then_custom_overlays_replace_complete_rows_like_cpp() {
        let entries = compose_effective_difficulty_entries_like_cpp(
            [entry(1, 1, 2, 3, 4), entry(2, 5, 6, 7, 8)],
            [entry(1, 9, 10, 11, 12), entry(3, 13, 14, 15, 16)],
            [entry(1, 17, 18, 19, 20), entry(3, 21, 22, 23, 24)],
            0xCB29_7E3A,
            &Db2HotfixRemovalStoreLikeCpp::default(),
        );

        assert_eq!(entries.get(&1), Some(&entry(1, 17, 18, 19, 20)));
        assert_eq!(entries.get(&2), Some(&entry(2, 5, 6, 7, 8)));
        assert_eq!(entries.get(&3), Some(&entry(3, 21, 22, 23, 24)));
    }

    #[test]
    fn hotfix_removals_run_after_all_difficulty_overlays_like_cpp() {
        let table_hash = 0xCB29_7E3A;
        let removals = Db2HotfixRemovalStoreLikeCpp::from_status_rows_like_cpp([
            (table_hash, 1, 2),
            (table_hash, 3, 2),
            (0xAABB_CCDD, 2, 2),
        ]);
        let entries = compose_effective_difficulty_entries_like_cpp(
            [entry(1, 1, 2, 3, 4), entry(2, 5, 6, 7, 8)],
            [entry(1, 9, 10, 11, 12)],
            [entry(3, 13, 14, 15, 16)],
            table_hash,
            &removals,
        );

        assert!(!entries.contains_key(&1));
        assert_eq!(entries.get(&2), Some(&entry(2, 5, 6, 7, 8)));
        assert!(!entries.contains_key(&3));
    }

    #[test]
    fn zero_id_and_zero_fallback_are_preserved_like_cpp() {
        let entries = compose_effective_difficulty_entries_like_cpp(
            [entry(0, 1, 9, 3, 4), entry(1, 5, 8, 7, 6)],
            [entry(0, 2, 0, 11, 12)],
            [entry(1, 13, 0, 15, 16)],
            0xCB29_7E3A,
            &Db2HotfixRemovalStoreLikeCpp::default(),
        );
        let mut store = DifficultyStore::from_entries(entries.into_values());
        store.table_hash_like_cpp = Some(0xCB29_7E3A);

        assert_eq!(store.fallback_difficulty_id_like_cpp(0), Some(0));
        assert_eq!(store.fallback_difficulty_id_like_cpp(1), Some(0));
        assert_eq!(store.table_hash_like_cpp(), Some(0xCB29_7E3A));
    }

    #[test]
    fn difficulty_fixture_exposes_cpp_table_hash() {
        let data_dir = "/home/server/woltk-server-core/Data";
        let locale = "enUS";
        let path = Path::new(data_dir)
            .join("dbc")
            .join(locale)
            .join("Difficulty.db2");
        if !path.exists() {
            eprintln!("Skipping test: Difficulty.db2 fixture not found");
            return;
        }

        let store = DifficultyStore::load(data_dir, locale)
            .unwrap_or_else(|error| panic!("failed to load Difficulty.db2: {error:#}"));
        assert_eq!(
            store.table_hash_like_cpp(),
            Some(0xCB29_7E3A),
            "the 3.4.3 fixture must expose the WDC4 table hash, not layout hash 0x3FE0C298"
        );
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
