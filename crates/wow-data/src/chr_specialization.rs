// Copyright (c) 2026 alseif0x
// RustyCore - WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 - https://www.gnu.org/licenses/gpl-3.0.html

//! ChrSpecialization.db2 reader.

use std::collections::HashMap;
use std::path::Path;

use anyhow::{Context, Result};
use tracing::info;

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
}

impl ChrSpecializationStore {
    pub fn from_entries(entries: impl IntoIterator<Item = ChrSpecializationEntry>) -> Self {
        Self {
            entries: entries.into_iter().map(|entry| (entry.id, entry)).collect(),
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
        Ok(Self { entries })
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
        self.entries
            .values()
            .find(|entry| entry.class_id == class_id && entry.order_index == order_index)
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
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
}
