// Copyright (c) 2026 alseif0x
// RustyCore - WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 - https://www.gnu.org/licenses/gpl-3.0.html

//! Minimal AreaTable.db2 reader for C++ phasing area-parent checks.

use std::collections::HashMap;
use std::path::Path;

use anyhow::{Context, Result};
use tracing::info;
use wow_database::{HotfixDatabase, HotfixStatements, WorldDatabase, WorldStatements};

use crate::wdc4::Wdc4Reader;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AreaTableEntry {
    pub id: u32,
    pub continent_id: u16,
    pub parent_area_id: u16,
    pub area_bit: i16,
    pub exploration_level: i8,
    pub mount_flags: i32,
    pub flags: u32,
}

#[derive(Debug, Clone, Default)]
pub struct AreaTableStore {
    entries: HashMap<u32, AreaTableEntry>,
}

#[derive(Debug, Clone, Default)]
pub struct FishingBaseSkillStoreLikeCpp {
    levels_by_area: HashMap<u32, i32>,
}

pub const AREA_FLAG_ALLOW_HEARTH_AND_RESURRECT_FROM_AREA_LIKE_CPP: u32 = 0x0800_0000;
pub const AREA_FLAG_IS_SUBZONE_LIKE_CPP: u32 = 0x4000_0000;

impl AreaTableEntry {
    /// C++ `Player::CheckAreaExploreAndOutdoor` derives
    /// `(offset, mask)` from `AreaTableEntry::AreaBit`.
    pub fn explored_zone_bit_like_cpp(&self, explored_zone_blocks: usize) -> Option<(usize, u64)> {
        let area_bit = usize::try_from(self.area_bit).ok()?;
        let offset = area_bit / 64;
        if offset >= explored_zone_blocks {
            return None;
        }

        Some((offset, 1u64 << (area_bit % 64)))
    }

    pub fn allow_hearth_and_resurrect_from_area_like_cpp(&self) -> bool {
        self.flags & AREA_FLAG_ALLOW_HEARTH_AND_RESURRECT_FROM_AREA_LIKE_CPP != 0
    }

    pub fn is_subzone_like_cpp(&self) -> bool {
        self.flags & AREA_FLAG_IS_SUBZONE_LIKE_CPP != 0
    }
}

impl AreaTableStore {
    pub fn from_entries(entries: impl IntoIterator<Item = AreaTableEntry>) -> Self {
        Self {
            entries: entries.into_iter().map(|entry| (entry.id, entry)).collect(),
        }
    }

    /// Load AreaTable.db2 from `{data_dir}/dbc/{locale}/AreaTable.db2`.
    ///
    /// C++ refs:
    /// - `DB2Structure.h::AreaTableEntry`
    /// - `DB2LoadInfo.h::AreaTableLoadInfo`
    /// - `ObjectMgr::LoadAreaPhases`
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        let path = Path::new(data_dir)
            .join("dbc")
            .join(locale)
            .join("AreaTable.db2");

        let reader = Wdc4Reader::open(&path)
            .with_context(|| format!("failed to open {}", path.display()))?;

        let mut entries = HashMap::with_capacity(reader.total_count());
        for (id, idx) in reader.iter_records() {
            entries.insert(
                id,
                AreaTableEntry {
                    id,
                    // WDC4 record ids supply C++ field 0 (`ID`), so
                    // `ContinentID` is DB2Meta field index 3 and `ParentAreaID` is index 4.
                    continent_id: reader.get_field_u16(idx, 3),
                    parent_area_id: reader.get_field_u16(idx, 4),
                    // C++ fields `AreaBit` and `ExplorationLevel`.
                    area_bit: reader.get_field_i16(idx, 5),
                    exploration_level: reader.get_field_i8(idx, 12),
                    // `MountFlags` is C++ field index 17, DB2Meta field index 16.
                    mount_flags: reader.get_field_i32(idx, 16),
                    // `Flags1` is C++ field index 22, DB2Meta field index 21
                    // when the record id supplies `ID`.
                    flags: reader.get_field_u32(idx, 21),
                },
            );
        }

        info!("Loaded {} areas from {}", entries.len(), path.display());
        Ok(Self { entries })
    }

    pub async fn load_with_hotfixes(
        data_dir: &str,
        locale: &str,
        hotfix_db: &HotfixDatabase,
    ) -> Result<Self> {
        let mut store = Self::load(data_dir, locale)?;
        let hotfix_rows = store.load_hotfix_rows(hotfix_db).await?;
        if hotfix_rows != 0 {
            info!("Loaded {hotfix_rows} AreaTable hotfix rows");
        }
        Ok(store)
    }

    async fn load_hotfix_rows(&mut self, db: &HotfixDatabase) -> Result<usize> {
        let stmt = db.prepare(HotfixStatements::SEL_AREA_TABLE);
        let mut result = db.query(&stmt).await?;
        if result.is_empty() {
            return Ok(0);
        }

        let mut count = 0usize;
        loop {
            let id: u32 = result.read(0);
            self.entries.insert(
                id,
                AreaTableEntry {
                    id,
                    continent_id: result.read(3),
                    parent_area_id: result.read(4),
                    area_bit: result.read(5),
                    exploration_level: result.read(12),
                    mount_flags: result.read(17),
                    flags: result.read(22),
                },
            );
            count += 1;

            if !result.next_row() {
                break;
            }
        }
        Ok(count)
    }

    pub fn get(&self, id: u32) -> Option<&AreaTableEntry> {
        self.entries.get(&id)
    }

    pub fn contains(&self, id: u32) -> bool {
        self.entries.contains_key(&id)
    }

    /// C++ `DB2Manager::IsInArea(objectAreaId, areaId)`.
    pub fn is_in_area_like_cpp(&self, mut object_area_id: u32, area_id: u32) -> bool {
        loop {
            if object_area_id == area_id {
                return true;
            }

            let Some(object_area) = self.get(object_area_id) else {
                return false;
            };

            object_area_id = u32::from(object_area.parent_area_id);
            if object_area_id == 0 {
                return false;
            }
        }
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

impl FishingBaseSkillStoreLikeCpp {
    pub fn from_entries(entries: impl IntoIterator<Item = (u32, i32)>) -> Self {
        Self {
            levels_by_area: entries.into_iter().collect(),
        }
    }

    /// C++ `ObjectMgr::LoadFishingBaseSkillLevel`.
    pub async fn load(db: &WorldDatabase, area_store: &AreaTableStore) -> Result<Self> {
        let stmt = db.prepare(WorldStatements::SEL_FISHING_BASE_SKILL_LEVELS);
        let mut result = db.query(&stmt).await?;
        if result.is_empty() {
            info!("Loaded 0 areas for fishing base skill level");
            return Ok(Self::default());
        }

        let mut levels_by_area = HashMap::new();
        loop {
            let area_id: u32 = result.read(0);
            let skill: i16 = result.read(1);
            if area_store.contains(area_id) {
                levels_by_area.insert(area_id, i32::from(skill));
            }

            if !result.next_row() {
                break;
            }
        }

        info!(
            "Loaded {} areas for fishing base skill level",
            levels_by_area.len()
        );
        Ok(Self { levels_by_area })
    }

    pub fn get(&self, area_id: u32) -> Option<i32> {
        self.levels_by_area.get(&area_id).copied()
    }

    /// C++ `ObjectMgr::GetFishingBaseSkillLevel`.
    pub fn base_skill_level_like_cpp(&self, area_store: &AreaTableStore, mut area_id: u32) -> i32 {
        while area_id != 0 {
            if let Some(skill) = self.get(area_id) {
                return skill;
            }

            let Some(area) = area_store.get(area_id) else {
                return 0;
            };
            area_id = u32::from(area.parent_area_id);
        }

        0
    }

    pub fn len(&self) -> usize {
        self.levels_by_area.len()
    }

    pub fn is_empty(&self) -> bool {
        self.levels_by_area.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn area_table_store_indexes_parent_area_like_cpp() {
        let store = AreaTableStore::from_entries([
            AreaTableEntry {
                id: 100,
                continent_id: 0,
                parent_area_id: 0,
                area_bit: -1,
                exploration_level: 0,
                mount_flags: 0,
                flags: 0,
            },
            AreaTableEntry {
                id: 101,
                continent_id: 0,
                parent_area_id: 100,
                area_bit: -1,
                exploration_level: 0,
                mount_flags: 0,
                flags: AREA_FLAG_IS_SUBZONE_LIKE_CPP,
            },
        ]);

        assert!(store.contains(100));
        assert_eq!(store.get(101).map(|area| area.parent_area_id), Some(100));
        assert!(store.is_in_area_like_cpp(101, 100));
        assert!(store.is_in_area_like_cpp(101, 101));
        assert!(!store.is_in_area_like_cpp(101, 999));
        assert_eq!(
            store.get(101).map(|area| area.is_subzone_like_cpp()),
            Some(true)
        );
    }

    #[test]
    fn fishing_base_skill_store_walks_parent_areas_like_cpp() {
        let areas = AreaTableStore::from_entries([
            AreaTableEntry {
                id: 10,
                continent_id: 0,
                parent_area_id: 0,
                area_bit: -1,
                exploration_level: 0,
                mount_flags: 0,
                flags: 0,
            },
            AreaTableEntry {
                id: 11,
                continent_id: 0,
                parent_area_id: 10,
                area_bit: -1,
                exploration_level: 0,
                mount_flags: 0,
                flags: AREA_FLAG_IS_SUBZONE_LIKE_CPP,
            },
        ]);
        let fishing = FishingBaseSkillStoreLikeCpp::from_entries([(10, 225)]);

        assert_eq!(fishing.base_skill_level_like_cpp(&areas, 11), 225);
        assert_eq!(fishing.base_skill_level_like_cpp(&areas, 999), 0);
    }

    #[test]
    fn area_table_entry_explored_zone_bit_matches_cpp_area_bit_math() {
        let entry = AreaTableEntry {
            id: 42,
            continent_id: 0,
            parent_area_id: 0,
            area_bit: 65,
            exploration_level: 12,
            mount_flags: 0,
            flags: 0,
        };

        assert_eq!(entry.explored_zone_bit_like_cpp(240), Some((1, 2)));
    }

    #[test]
    fn area_table_entry_invalid_area_bit_is_not_discoverable_like_cpp() {
        let negative = AreaTableEntry {
            id: 43,
            continent_id: 0,
            parent_area_id: 0,
            area_bit: -1,
            exploration_level: 0,
            mount_flags: 0,
            flags: 0,
        };
        let out_of_range = AreaTableEntry {
            area_bit: 240 * 64,
            ..negative
        };

        assert_eq!(negative.explored_zone_bit_like_cpp(240), None);
        assert_eq!(out_of_range.explored_zone_bit_like_cpp(240), None);
    }

    #[test]
    fn load_area_table_db2_when_fixture_exists() {
        let data_dir = "/home/server/woltk-server-core/Data";
        let locale = "esES";
        let path = Path::new(data_dir)
            .join("dbc")
            .join(locale)
            .join("AreaTable.db2");
        if !path.exists() {
            eprintln!(
                "Skipping test: AreaTable.db2 not found at {}",
                path.display()
            );
            return;
        }

        let store = AreaTableStore::load(data_dir, locale).expect("failed to load AreaTable.db2");
        assert!(!store.is_empty());
    }
}
