// Copyright (c) 2026 alseif0x
// RustyCore - WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 - https://www.gnu.org/licenses/gpl-3.0.html

//! ChrSpecialization.db2 reader.

use std::collections::HashMap;
use std::path::Path;

use anyhow::{Context, Result};
use tracing::info;
use wow_database::{HotfixDatabase, HotfixStatements, SqlResult};

use crate::Db2HotfixRemovalStoreLikeCpp;
use crate::wdc4::Wdc4Reader;

/// Minimal C++ `ChrSpecializationEntry` fields needed by loot-spec validation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChrSpecializationEntry {
    pub id: u32,
    pub class_id: u8,
    pub order_index: i8,
    pub role: i8,
}

/// In-memory store for `ChrSpecialization.db2`.
#[derive(Debug)]
pub struct ChrSpecializationStore {
    entries: HashMap<u32, ChrSpecializationEntry>,
    by_class_and_index_like_cpp: HashMap<(u8, i8), ChrSpecializationEntry>,
    table_hash_like_cpp: Option<u32>,
}

impl ChrSpecializationStore {
    pub fn from_entries(entries: impl IntoIterator<Item = ChrSpecializationEntry>) -> Self {
        let entries: HashMap<_, _> = entries.into_iter().map(|entry| (entry.id, entry)).collect();
        Self {
            by_class_and_index_like_cpp: build_class_index_before_removals_like_cpp(&entries),
            entries,
            table_hash_like_cpp: None,
        }
    }

    /// Load ChrSpecialization.db2 from `{data_dir}/dbc/{locale}/ChrSpecialization.db2`.
    ///
    /// C++ refs:
    /// - `DB2Structure.h::ChrSpecializationEntry`
    /// - `DB2LoadInfo.h::ChrSpecializationLoadInfo`
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        let path = Path::new(data_dir)
            .join("dbc")
            .join(locale)
            .join("ChrSpecialization.db2");

        let reader = Wdc4Reader::open(&path)
            .with_context(|| format!("failed to open {}", path.display()))?;

        let mut entries = HashMap::with_capacity(reader.total_count());
        for (id, idx) in reader.iter_records() {
            entries.insert(
                id,
                ChrSpecializationEntry {
                    id,
                    class_id: reader.get_field_u8(idx, 4),
                    order_index: reader.get_field_i8(idx, 5),
                    role: reader.get_field_i8(idx, 7),
                },
            );
        }

        info!(
            "Loaded {} chr specialization rows from {}",
            entries.len(),
            path.display()
        );
        Ok(Self {
            by_class_and_index_like_cpp: build_class_index_before_removals_like_cpp(&entries),
            entries,
            table_hash_like_cpp: Some(reader.table_hash()),
        })
    }

    /// Load the effective C++ `sChrSpecializationStore` authority.
    ///
    /// C++ refs:
    /// - `DB2StorageBase::LoadFromDB`: WDC4, official SQL, then custom SQL;
    /// - `HotfixDatabase.cpp::HOTFIX_SEL_CHR_SPECIALIZATION`: SQL field order;
    /// - `DB2Manager::LoadHotfixData`: final `(TableHash, RecordID)` removals.
    pub async fn load_effective_like_cpp(
        data_dir: &str,
        locale: &str,
        hotfix_db: &HotfixDatabase,
        removals: &Db2HotfixRemovalStoreLikeCpp,
    ) -> Result<Self> {
        let mut store = Self::load(data_dir, locale)?;
        let table_hash = store
            .table_hash_like_cpp
            .context("ChrSpecialization.db2 is missing its WDC4 table hash")?;
        let mut overlay_batches = [Vec::new(), Vec::new()];

        // `DB2StorageBase::LoadFromDB` calls `Load(false)` before
        // `Load(true)`. The loader binds `!custom`, so official
        // (`VerifiedBuild > 0`) records precede custom records.
        for (batch_index, official) in [true, false].into_iter().enumerate() {
            let mut stmt = hotfix_db.prepare(HotfixStatements::SEL_CHR_SPECIALIZATION);
            stmt.set_bool(0, official);
            let mut result = hotfix_db
                .query(&stmt)
                .await
                .context("failed to load ChrSpecialization.db2 SQL overlay")?;
            if result.is_empty() {
                continue;
            }

            loop {
                overlay_batches[batch_index].push(ChrSpecializationEntry {
                    id: read_u32_checked_like_cpp(&result, 3, "ChrSpecialization.ID")?,
                    class_id: read_u8_checked_like_cpp(&result, 4, "ChrSpecialization.ClassID")?,
                    order_index: read_i8_checked_like_cpp(
                        &result,
                        5,
                        "ChrSpecialization.OrderIndex",
                    )?,
                    role: read_i8_checked_like_cpp(&result, 7, "ChrSpecialization.Role")?,
                });
                if !result.next_row() {
                    break;
                }
            }
        }

        let [official_overlay_entries, custom_overlay_entries] = overlay_batches;
        let (entries, by_class_and_index_like_cpp) =
            compose_effective_chr_specialization_entries_like_cpp(
                std::mem::take(&mut store.entries).into_values(),
                official_overlay_entries,
                custom_overlay_entries,
                table_hash,
                removals,
            );
        store.entries = entries;
        store.by_class_and_index_like_cpp = by_class_and_index_like_cpp;
        Ok(store)
    }

    pub fn get(&self, id: u32) -> Option<&ChrSpecializationEntry> {
        self.entries.get(&id)
    }

    /// C++ `DB2Manager::GetChrSpecializationByIndex`: resolve the class-local
    /// specialization slot populated from `ChrSpecializationEntry::OrderIndex`.
    pub fn get_by_class_and_index_like_cpp(
        &self,
        class_id: u8,
        index: u8,
    ) -> Option<&ChrSpecializationEntry> {
        let order_index = i8::try_from(index).ok()?;
        self.by_class_and_index_like_cpp
            .get(&(class_id, order_index))
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
}

fn compose_effective_chr_specialization_entries_like_cpp(
    base_entries: impl IntoIterator<Item = ChrSpecializationEntry>,
    official_overlay_entries: impl IntoIterator<Item = ChrSpecializationEntry>,
    custom_overlay_entries: impl IntoIterator<Item = ChrSpecializationEntry>,
    table_hash: u32,
    removals: &Db2HotfixRemovalStoreLikeCpp,
) -> (
    HashMap<u32, ChrSpecializationEntry>,
    HashMap<(u8, i8), ChrSpecializationEntry>,
) {
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

    // C++ builds `_chrSpecializationsByIndex` immediately after DB2 plus SQL
    // overlays, before `DB2Manager::LoadHotfixData` erases primary-store
    // records. Keep that derived snapshot even when the direct ID view below
    // receives a final tombstone.
    let by_class_and_index_like_cpp =
        build_class_index_before_removals_like_cpp(&effective_entries);
    effective_entries
        .retain(|record_id, _| !removals.contains_like_cpp(table_hash, *record_id as i32));
    (effective_entries, by_class_and_index_like_cpp)
}

fn build_class_index_before_removals_like_cpp(
    entries: &HashMap<u32, ChrSpecializationEntry>,
) -> HashMap<(u8, i8), ChrSpecializationEntry> {
    let mut record_ids = entries.keys().copied().collect::<Vec<_>>();
    record_ids.sort_unstable();

    let mut by_class_and_index = HashMap::new();
    for record_id in record_ids {
        let entry = entries[&record_id];
        // `DBStorageIterator` visits record IDs in ascending order, so a later
        // (higher-ID) duplicate class/order slot replaces the earlier one.
        // `ChrSpecializationFlag::PetOverrideSpec` remapping is outside this
        // minimal entry, which intentionally represents the ordinary ClassID
        // path used for player specialization lookup.
        by_class_and_index.insert((entry.class_id, entry.order_index), entry);
    }
    by_class_and_index
}

fn read_integer_checked_like_cpp(
    result: &SqlResult,
    column: usize,
    field: &'static str,
) -> Result<i128> {
    result
        .try_read::<i64>(column)
        .map(i128::from)
        .or_else(|| result.try_read::<u64>(column).map(i128::from))
        .or_else(|| result.try_read::<i32>(column).map(i128::from))
        .or_else(|| result.try_read::<u32>(column).map(i128::from))
        .or_else(|| result.try_read::<i16>(column).map(i128::from))
        .or_else(|| result.try_read::<u16>(column).map(i128::from))
        .or_else(|| result.try_read::<i8>(column).map(i128::from))
        .or_else(|| result.try_read::<u8>(column).map(i128::from))
        .with_context(|| format!("missing or non-integer {field} SQL column {column}"))
}

fn read_u32_checked_like_cpp(
    result: &SqlResult,
    column: usize,
    field: &'static str,
) -> Result<u32> {
    let value = read_integer_checked_like_cpp(result, column, field)?;
    u32::try_from(value).with_context(|| format!("{field} SQL value {value} is not u32"))
}

fn read_u8_checked_like_cpp(result: &SqlResult, column: usize, field: &'static str) -> Result<u8> {
    let value = read_integer_checked_like_cpp(result, column, field)?;
    u8::try_from(value).with_context(|| format!("{field} SQL value {value} is not u8"))
}

fn read_i8_checked_like_cpp(result: &SqlResult, column: usize, field: &'static str) -> Result<i8> {
    let value = read_integer_checked_like_cpp(result, column, field)?;
    i8::try_from(value).with_context(|| format!("{field} SQL value {value} is not i8"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chr_specialization_store_indexes_by_id_like_cpp_store() {
        let store = ChrSpecializationStore::from_entries([ChrSpecializationEntry {
            id: 65,
            class_id: 2,
            order_index: 0,
            role: 1,
        }]);

        assert_eq!(store.get(65).unwrap().class_id, 2);
        assert_eq!(store.get(65).unwrap().role, 1);
        assert!(store.get(66).is_none());
    }

    #[test]
    fn chr_specialization_store_resolves_class_local_order_index_like_cpp() {
        let store = ChrSpecializationStore::from_entries([
            ChrSpecializationEntry {
                id: 65,
                class_id: 2,
                order_index: 0,
                role: 1,
            },
            ChrSpecializationEntry {
                id: 71,
                class_id: 1,
                order_index: 0,
                role: 2,
            },
        ]);

        assert_eq!(
            store
                .get_by_class_and_index_like_cpp(2, 0)
                .map(|entry| entry.id),
            Some(65)
        );
        assert!(store.get_by_class_and_index_like_cpp(2, 1).is_none());
    }

    #[test]
    fn class_index_collision_is_resolved_by_highest_record_id_like_cpp_iteration() {
        let store = ChrSpecializationStore::from_entries([
            ChrSpecializationEntry {
                id: 100,
                class_id: 2,
                order_index: 0,
                role: 1,
            },
            ChrSpecializationEntry {
                id: 50,
                class_id: 2,
                order_index: 0,
                role: 2,
            },
        ]);

        assert_eq!(
            store
                .get_by_class_and_index_like_cpp(2, 0)
                .map(|entry| entry.id),
            Some(100)
        );
    }

    #[test]
    fn db2_official_custom_and_removal_order_builds_effective_class_index_like_cpp() {
        let table_hash = 0xA00F_8E60;
        let removals = Db2HotfixRemovalStoreLikeCpp::from_status_rows_like_cpp([
            (table_hash, 71, 2),
            (table_hash, 72, 2),
            (0xAABB_CCDD, 66, 2),
        ]);
        let (entries, by_class_and_index_like_cpp) =
            compose_effective_chr_specialization_entries_like_cpp(
                [
                    ChrSpecializationEntry {
                        id: 65,
                        class_id: 1,
                        order_index: 0,
                        role: 0,
                    },
                    ChrSpecializationEntry {
                        id: 71,
                        class_id: 1,
                        order_index: 1,
                        role: 2,
                    },
                ],
                [
                    ChrSpecializationEntry {
                        id: 65,
                        class_id: 2,
                        order_index: 0,
                        role: 1,
                    },
                    ChrSpecializationEntry {
                        id: 66,
                        class_id: 2,
                        order_index: 1,
                        role: 2,
                    },
                    ChrSpecializationEntry {
                        id: 70,
                        class_id: 2,
                        order_index: 2,
                        role: 1,
                    },
                ],
                [
                    ChrSpecializationEntry {
                        id: 70,
                        class_id: 2,
                        order_index: 2,
                        role: 0,
                    },
                    ChrSpecializationEntry {
                        id: 72,
                        class_id: 3,
                        order_index: 0,
                        role: 0,
                    },
                ],
                table_hash,
                &removals,
            );
        let store = ChrSpecializationStore {
            entries,
            by_class_and_index_like_cpp,
            table_hash_like_cpp: Some(table_hash),
        };

        assert_eq!(store.len(), 3);
        assert_eq!(store.get(65).map(|entry| entry.class_id), Some(2));
        assert_eq!(store.get(65).map(|entry| entry.role), Some(1));
        assert_eq!(store.get(70).map(|entry| entry.role), Some(0));
        assert!(store.get(71).is_none());
        assert!(store.get(72).is_none());
        assert_eq!(
            store
                .get_by_class_and_index_like_cpp(2, 0)
                .map(|entry| entry.id),
            Some(65)
        );
        assert_eq!(
            store
                .get_by_class_and_index_like_cpp(2, 1)
                .map(|entry| entry.id),
            Some(66)
        );
        assert_eq!(
            store
                .get_by_class_and_index_like_cpp(2, 2)
                .map(|entry| entry.id),
            Some(70)
        );
        assert!(store.get_by_class_and_index_like_cpp(1, 0).is_none());
        assert_eq!(
            store
                .get_by_class_and_index_like_cpp(1, 1)
                .map(|entry| entry.id),
            Some(71),
            "C++ builds the derived class/index snapshot before final tombstones"
        );
        assert_eq!(
            store
                .get_by_class_and_index_like_cpp(3, 0)
                .map(|entry| entry.id),
            Some(72),
            "custom rows remain in C++'s derived snapshot after direct-ID removal"
        );
    }

    #[test]
    fn chr_specialization_fixture_exposes_cpp_table_hash() {
        let data_dir = "/home/server/woltk-server-core/Data";
        let locale = "esES";
        let path = Path::new(data_dir)
            .join("dbc")
            .join(locale)
            .join("ChrSpecialization.db2");
        if !path.exists() {
            eprintln!("Skipping test: ChrSpecialization.db2 fixture not found");
            return;
        }

        let store = ChrSpecializationStore::load(data_dir, locale)
            .unwrap_or_else(|error| panic!("failed to load ChrSpecialization.db2: {error:#}"));
        assert_eq!(
            store.table_hash_like_cpp(),
            Some(0xA00F_8E60),
            "the fixture must expose the WDC4 table hash, not layout hash 0x1F1A9A8F"
        );
    }
}
