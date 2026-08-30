// Copyright (c) 2026 alseif0x
// RustyCore - WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 - https://www.gnu.org/licenses/gpl-3.0.html

//! ChrSpecialization.db2 reader.

use std::collections::HashMap;
use std::path::Path;

use anyhow::{Context, Result};
use tracing::info;

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

    /// Apply the already decoded official/custom SQL overlays and final
    /// tombstones to a WDC4-backed store in C++ order.
    ///
    /// C++ refs:
    /// - `DB2Store.cpp::DB2StorageBase::LoadFromDB` supplies official then custom;
    /// - `DB2Stores.cpp::DB2Manager::LoadStores` builds the class/order index;
    /// - `DB2Stores.cpp::DB2Manager::LoadHotfixData` erases final tombstones.
    pub fn apply_hotfix_overlays_like_cpp(
        mut self,
        official_overlay_entries: impl IntoIterator<Item = ChrSpecializationEntry>,
        custom_overlay_entries: impl IntoIterator<Item = ChrSpecializationEntry>,
        removals: &Db2HotfixRemovalStoreLikeCpp,
    ) -> Result<Self> {
        let table_hash = self
            .table_hash_like_cpp
            .context("ChrSpecialization.db2 is missing its WDC4 table hash")?;
        let (entries, by_class_and_index_like_cpp) =
            compose_effective_chr_specialization_entries_like_cpp(
                std::mem::take(&mut self.entries).into_values(),
                official_overlay_entries,
                custom_overlay_entries,
                table_hash,
                removals,
            );
        self.entries = entries;
        self.by_class_and_index_like_cpp = by_class_and_index_like_cpp;
        Ok(self)
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
