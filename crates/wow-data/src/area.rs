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
    faction_group_masks: HashMap<u32, u8>,
}

#[derive(Debug, Clone, Default)]
pub struct FishingBaseSkillStoreLikeCpp {
    levels_by_area: HashMap<u32, i32>,
}

pub const AREA_FLAG_ALLOW_HEARTH_AND_RESURRECT_FROM_AREA_LIKE_CPP: u32 = 0x0800_0000;
pub const AREA_FLAG_LINKED_CHAT_LIKE_CPP: u32 = 0x0000_0100;
pub const AREA_FLAG_NO_PVP_LIKE_CPP: u32 = 0x0000_0800;
pub const AREA_FLAG_HORDE_RESTING_LIKE_CPP: u32 = 0x0040_0000;
pub const AREA_FLAG_ALLIANCE_RESTING_LIKE_CPP: u32 = 0x0080_0000;
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

    pub fn linked_chat_like_cpp(&self) -> bool {
        self.flags & AREA_FLAG_LINKED_CHAT_LIKE_CPP != 0
    }

    pub fn is_sanctuary_like_cpp(&self) -> bool {
        self.flags & AREA_FLAG_NO_PVP_LIKE_CPP != 0
    }

    pub fn alliance_resting_like_cpp(&self) -> bool {
        self.flags & AREA_FLAG_ALLIANCE_RESTING_LIKE_CPP != 0
    }

    pub fn horde_resting_like_cpp(&self) -> bool {
        self.flags & AREA_FLAG_HORDE_RESTING_LIKE_CPP != 0
    }

    pub fn is_subzone_like_cpp(&self) -> bool {
        self.flags & AREA_FLAG_IS_SUBZONE_LIKE_CPP != 0
    }
}

impl AreaTableStore {
    pub fn from_entries(entries: impl IntoIterator<Item = AreaTableEntry>) -> Self {
        Self {
            entries: entries.into_iter().map(|entry| (entry.id, entry)).collect(),
            faction_group_masks: HashMap::new(),
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
        let mut faction_group_masks = HashMap::with_capacity(reader.total_count());
        for (id, idx) in reader.iter_records() {
            // `AreaTableMeta` has an external ID (`IndexField = -1`), so its
            // 23 physical WDC4 fields are zero-based without `ID`:
            // `ContinentID`, `ParentAreaID`, `AreaBit`, `ExplorationLevel`, and
            // `FactionGroupMask` are fields 2, 3, 4, 11, and 14 respectively.
            // The hotfix SELECT below includes `ID` as column 0, so each of
            // those values appears one column later there.
            faction_group_masks.insert(id, reader.get_field_u8(idx, 14));
            entries.insert(
                id,
                AreaTableEntry {
                    id,
                    continent_id: reader.get_field_u16(idx, 2),
                    parent_area_id: reader.get_field_u16(idx, 3),
                    // C++ fields `AreaBit` and `ExplorationLevel`.
                    area_bit: reader.get_field_i16(idx, 4),
                    exploration_level: reader.get_field_i8(idx, 11),
                    // `MountFlags` is C++ field index 17, DB2Meta field index 16.
                    mount_flags: reader.get_field_i32(idx, 16),
                    // `Flags1` is C++ field index 22, DB2Meta field index 21
                    // when the record id supplies `ID`.
                    flags: reader.get_field_u32(idx, 21),
                },
            );
        }

        info!("Loaded {} areas from {}", entries.len(), path.display());
        Ok(Self {
            entries,
            faction_group_masks,
        })
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
            self.faction_group_masks.insert(id, result.read(15));
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

    /// C++ `AreaTableEntry::FactionGroupMask` (DB2 field 15).
    pub fn faction_group_mask_like_cpp(&self, id: u32) -> u8 {
        self.faction_group_masks.get(&id).copied().unwrap_or(0)
    }

    pub fn set_faction_group_mask_like_cpp(&mut self, id: u32, mask: u8) {
        if self.entries.contains_key(&id) {
            self.faction_group_masks.insert(id, mask);
        }
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

    /// C++ `DB2Manager::IsInArea` walks the AreaTable parent chain. This helper exposes the
    /// same chain for callers that already evaluate area predicates from a prebuilt context.
    pub fn parent_area_ids_like_cpp(&self, mut area_id: u32) -> Vec<u32> {
        let mut parents = Vec::new();
        while let Some(area) = self.get(area_id) {
            let parent = u32::from(area.parent_area_id);
            if parent == 0 {
                break;
            }

            parents.push(parent);
            area_id = parent;
        }
        parents
    }

    /// C++ `PlayerCondition::Explored` looks up each requested AreaTable row and then checks
    /// `Player::m_activePlayerData->ExploredZones` using `AreaTableEntry::AreaBit`.
    pub fn explored_area_ids_from_blocks_like_cpp(&self, explored_zone_blocks: &[u64]) -> Vec<u16> {
        let mut explored = Vec::new();
        for entry in self.entries.values() {
            let Some((offset, mask)) = entry.explored_zone_bit_like_cpp(explored_zone_blocks.len())
            else {
                continue;
            };

            if explored_zone_blocks[offset] & mask != 0 {
                if let Ok(area_id) = u16::try_from(entry.id) {
                    explored.push(area_id);
                }
            }
        }
        explored.sort_unstable();
        explored.dedup();
        explored
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

    fn push_u16_le(bytes: &mut Vec<u8>, value: u16) {
        bytes.extend_from_slice(&value.to_le_bytes());
    }

    fn push_u32_le(bytes: &mut Vec<u8>, value: u32) {
        bytes.extend_from_slice(&value.to_le_bytes());
    }

    fn push_u64_le(bytes: &mut Vec<u8>, value: u64) {
        bytes.extend_from_slice(&value.to_le_bytes());
    }

    /// One fixed-size AreaTable WDC4 row with an external ID. The adjacent
    /// field 15 deliberately has low byte 0x9A so reading AmbientMultiplier
    /// instead of physical field 14 cannot accidentally produce mask 6.
    fn minimal_area_table_wdc4() -> Vec<u8> {
        const FIELD_COUNT: u32 = 23;
        const RECORD_SIZE: u32 = FIELD_COUNT * 4;
        const HEADER_SIZE: u32 = 72;
        const SECTION_HEADER_SIZE: u32 = 40;
        const FIELD_META_SIZE: u32 = 4;
        const FIELD_STORAGE_INFO_SIZE: u32 = 24;
        const RECORD_OFFSET: u32 = HEADER_SIZE
            + SECTION_HEADER_SIZE
            + FIELD_COUNT * FIELD_META_SIZE
            + FIELD_COUNT * FIELD_STORAGE_INFO_SIZE;

        let mut bytes = Vec::new();
        // WDC4 header.
        push_u32_le(&mut bytes, 0x3443_4457);
        push_u32_le(&mut bytes, 1); // record_count
        push_u32_le(&mut bytes, FIELD_COUNT);
        push_u32_le(&mut bytes, RECORD_SIZE);
        push_u32_le(&mut bytes, 0); // string_table_size
        push_u32_le(&mut bytes, 0); // table_hash (not used by this loader)
        push_u32_le(&mut bytes, 0x19CA_1DC6); // C++ AreaTableMeta layout
        push_u32_le(&mut bytes, 4395); // min_id
        push_u32_le(&mut bytes, 4395); // max_id
        push_u32_le(&mut bytes, 0); // locale
        push_u16_le(&mut bytes, 0x04); // external ID list
        push_u16_le(&mut bytes, u16::MAX); // no inline ID field
        push_u32_le(&mut bytes, FIELD_COUNT);
        push_u32_le(&mut bytes, 0); // packed_data_offset
        push_u32_le(&mut bytes, 0); // lookup_column_count
        push_u32_le(&mut bytes, FIELD_COUNT * FIELD_STORAGE_INFO_SIZE);
        push_u32_le(&mut bytes, 0); // common_data_size
        push_u32_le(&mut bytes, 0); // pallet_data_size
        push_u32_le(&mut bytes, 1); // section_count

        // One fixed-size section followed by one external record ID.
        push_u64_le(&mut bytes, 0); // tact_key_hash
        push_u32_le(&mut bytes, RECORD_OFFSET);
        push_u32_le(&mut bytes, 1); // record_count
        push_u32_le(&mut bytes, 0); // string_table_size
        push_u32_le(&mut bytes, RECORD_OFFSET + RECORD_SIZE);
        push_u32_le(&mut bytes, 4); // id_list_size
        push_u32_le(&mut bytes, 0); // relationship_data_size
        push_u32_le(&mut bytes, 0); // offset_map_id_count
        push_u32_le(&mut bytes, 0); // copy_table_count

        // Field metadata is intentionally opaque to Wdc4Reader; storage info
        // below supplies the physical bit offsets and widths.
        bytes.resize(
            bytes.len() + FIELD_COUNT as usize * FIELD_META_SIZE as usize,
            0,
        );
        for field in 0..FIELD_COUNT {
            push_u16_le(&mut bytes, (field * 32) as u16);
            push_u16_le(&mut bytes, 32);
            push_u32_le(&mut bytes, 0); // additional_data_size
            push_u32_le(&mut bytes, 0); // CompressionType::None
            push_u32_le(&mut bytes, 0);
            push_u32_le(&mut bytes, 0);
            push_u32_le(&mut bytes, 0);
        }
        assert_eq!(bytes.len(), RECORD_OFFSET as usize);

        let mut fields = [0u32; FIELD_COUNT as usize];
        fields[2] = 530; // C++ AreaTableEntry::ContinentID
        fields[3] = 3519; // C++ AreaTableEntry::ParentAreaID
        fields[4] = 1321; // C++ AreaTableEntry::AreaBit
        fields[11] = 64; // C++ AreaTableEntry::ExplorationLevel
        fields[14] = 6; // C++ AreaTableEntry::FactionGroupMask
        fields[15] = 0.3f32.to_bits(); // AmbientMultiplier, low byte 0x9A
        for field in fields {
            push_u32_le(&mut bytes, field);
        }
        push_u32_le(&mut bytes, 4395); // external Dalaran record ID
        bytes
    }

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
    fn area_table_store_derives_parent_chain_like_cpp() {
        let store = AreaTableStore::from_entries([
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
            AreaTableEntry {
                id: 12,
                continent_id: 0,
                parent_area_id: 11,
                area_bit: -1,
                exploration_level: 0,
                mount_flags: 0,
                flags: AREA_FLAG_IS_SUBZONE_LIKE_CPP,
            },
        ]);

        assert_eq!(store.parent_area_ids_like_cpp(12), vec![11, 10]);
        assert!(store.parent_area_ids_like_cpp(999).is_empty());
    }

    #[test]
    fn area_table_store_derives_explored_area_ids_from_blocks_like_cpp() {
        let store = AreaTableStore::from_entries([
            AreaTableEntry {
                id: 10,
                continent_id: 0,
                parent_area_id: 0,
                area_bit: 0,
                exploration_level: 0,
                mount_flags: 0,
                flags: 0,
            },
            AreaTableEntry {
                id: 11,
                continent_id: 0,
                parent_area_id: 0,
                area_bit: 65,
                exploration_level: 0,
                mount_flags: 0,
                flags: 0,
            },
            AreaTableEntry {
                id: 70_000,
                continent_id: 0,
                parent_area_id: 0,
                area_bit: 1,
                exploration_level: 0,
                mount_flags: 0,
                flags: 0,
            },
            AreaTableEntry {
                id: 12,
                continent_id: 0,
                parent_area_id: 0,
                area_bit: -1,
                exploration_level: 0,
                mount_flags: 0,
                flags: 0,
            },
        ]);
        let mut blocks = [0u64; 2];
        blocks[0] = 1;
        blocks[1] = 2;

        assert_eq!(
            store.explored_area_ids_from_blocks_like_cpp(&blocks),
            vec![10, 11]
        );
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
        assert_eq!(
            store.get(3697),
            Some(&AreaTableEntry {
                id: 3697,
                continent_id: 530,
                parent_area_id: 3519,
                area_bit: 1321,
                exploration_level: 64,
                mount_flags: 2,
                flags: 0x4000_C440,
            }),
            "Shattrath City"
        );
        assert_eq!(store.faction_group_mask_like_cpp(1519), 2, "Stormwind");
        assert_eq!(store.faction_group_mask_like_cpp(1637), 4, "Orgrimmar");
        assert_eq!(store.faction_group_mask_like_cpp(4395), 6, "Dalaran");
    }

    #[test]
    fn base_loader_reads_physical_area_fields_without_sql_id_column() {
        let root = std::env::temp_dir().join(format!(
            "rustycore-area-table-wdc4-field-test-{}",
            std::process::id()
        ));
        let fixture_dir = root.join("dbc").join("enUS");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&fixture_dir).expect("create WDC4 fixture directory");
        std::fs::write(fixture_dir.join("AreaTable.db2"), minimal_area_table_wdc4())
            .expect("write minimal AreaTable WDC4 fixture");

        let store = AreaTableStore::load(root.to_str().expect("UTF-8 temp path"), "enUS")
            .expect("load minimal AreaTable WDC4 fixture");
        assert_eq!(
            store.get(4395),
            Some(&AreaTableEntry {
                id: 4395,
                continent_id: 530,
                parent_area_id: 3519,
                area_bit: 1321,
                exploration_level: 64,
                mount_flags: 0,
                flags: 0,
            })
        );
        assert_eq!(store.faction_group_mask_like_cpp(4395), 6);

        std::fs::remove_dir_all(root).expect("remove WDC4 fixture directory");
    }
}
