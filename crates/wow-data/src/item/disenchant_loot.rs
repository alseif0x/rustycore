// Copyright (c) 2026 alseif0x
// RustyCore - WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 - https://www.gnu.org/licenses/gpl-3.0.html

//! ItemDisenchantLoot.db2 reader.

use std::collections::HashMap;
use std::path::Path;

use anyhow::{Context, Result};
use tracing::info;

use crate::wdc4::Wdc4Reader;

/// C++ `ItemDisenchantLootEntry`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ItemDisenchantLootEntry {
    pub id: u32,
    pub subclass: i8,
    pub quality: u8,
    pub min_level: u16,
    pub max_level: u16,
    pub skill_required: u16,
    pub expansion_id: i8,
    pub class_id: u32,
}

/// In-memory store for `ItemDisenchantLoot.db2`.
pub struct ItemDisenchantLootStore {
    entries_by_id: HashMap<u32, ItemDisenchantLootEntry>,
    ordered_entries: Vec<ItemDisenchantLootEntry>,
}

impl ItemDisenchantLootStore {
    pub fn from_entries(entries: impl IntoIterator<Item = ItemDisenchantLootEntry>) -> Self {
        let ordered_entries: Vec<_> = entries.into_iter().collect();
        let entries_by_id = ordered_entries
            .iter()
            .copied()
            .map(|entry| (entry.id, entry))
            .collect();

        Self {
            entries_by_id,
            ordered_entries,
        }
    }

    /// Load ItemDisenchantLoot.db2 from `{data_dir}/dbc/{locale}/ItemDisenchantLoot.db2`.
    ///
    /// C++ refs:
    /// - `DB2Structure.h::ItemDisenchantLootEntry`
    /// - `DB2LoadInfo.h::ItemDisenchantLootLoadInfo`
    /// - `DB2Stores.cpp::sItemDisenchantLootStore`
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        let path = Path::new(data_dir)
            .join("dbc")
            .join(locale)
            .join("ItemDisenchantLoot.db2");

        let reader = Wdc4Reader::open(&path)
            .with_context(|| format!("failed to open {}", path.display()))?;

        let mut ordered_entries = Vec::with_capacity(reader.total_count());
        for (id, idx) in reader.iter_records() {
            let class_id = reader
                .get_relationship_id(idx)
                .with_context(|| format!("missing WDC4 relationship Class for record {id}"))?;
            ordered_entries.push(ItemDisenchantLootEntry {
                id,
                subclass: reader.get_field_i8(idx, 0),
                quality: reader.get_field_u8(idx, 1),
                min_level: reader.get_field_u16(idx, 2),
                max_level: reader.get_field_u16(idx, 3),
                skill_required: reader.get_field_u16(idx, 4),
                expansion_id: reader.get_field_i8(idx, 5),
                class_id,
            });
        }

        let entries_by_id = ordered_entries
            .iter()
            .copied()
            .map(|entry| (entry.id, entry))
            .collect();

        info!(
            "Loaded {} item disenchant loot rows from {}",
            ordered_entries.len(),
            path.display()
        );
        Ok(Self {
            entries_by_id,
            ordered_entries,
        })
    }

    pub fn get(&self, id: u32) -> Option<&ItemDisenchantLootEntry> {
        self.entries_by_id.get(&id)
    }

    pub fn iter(&self) -> impl Iterator<Item = &ItemDisenchantLootEntry> {
        self.ordered_entries.iter()
    }

    /// Match the reusable DB2 row filters used by C++ `Item::GetDisenchantLoot`.
    ///
    /// This intentionally preserves this Trinity branch's subclass guard shape:
    /// `if (disenchant->Subclass >= 0 && itemSubClass) continue;`.
    /// The surrounding item-template gates live outside this DB2 store.
    pub fn find_for_item_like_cpp(
        &self,
        item_class: u32,
        item_subclass: i8,
        quality: u8,
        item_level: u32,
        expansion: u8,
    ) -> Option<&ItemDisenchantLootEntry> {
        self.ordered_entries.iter().find(|entry| {
            if entry.class_id != item_class {
                return false;
            }

            if entry.subclass >= 0 && item_subclass != 0 {
                return false;
            }

            if entry.quality != quality {
                return false;
            }

            let min_level = u32::from(entry.min_level);
            let max_level = u32::from(entry.max_level);
            if min_level > item_level || max_level < item_level {
                return false;
            }

            entry.expansion_id == -2 || entry.expansion_id == expansion as i8
        })
    }

    pub fn len(&self) -> usize {
        self.ordered_entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.ordered_entries.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(
        id: u32,
        class_id: u32,
        quality: u8,
        min_level: u16,
        max_level: u16,
    ) -> ItemDisenchantLootEntry {
        ItemDisenchantLootEntry {
            id,
            subclass: -1,
            quality,
            min_level,
            max_level,
            skill_required: 175,
            expansion_id: -2,
            class_id,
        }
    }

    #[test]
    fn item_disenchant_loot_store_indexes_by_id_and_keeps_iteration_order() {
        let store = ItemDisenchantLootStore::from_entries([
            entry(20, 4, 2, 10, 20),
            entry(10, 2, 3, 30, 40),
        ]);

        assert_eq!(store.get(10).unwrap().class_id, 2);
        assert_eq!(
            store.iter().map(|entry| entry.id).collect::<Vec<_>>(),
            vec![20, 10]
        );
        assert!(store.get(30).is_none());
    }

    #[test]
    fn item_disenchant_loot_matcher_preserves_cpp_filters() {
        let mut subclass_specific = entry(1, 4, 2, 20, 40);
        subclass_specific.subclass = 1;
        let store = ItemDisenchantLootStore::from_entries([
            subclass_specific,
            entry(2, 4, 2, 20, 40),
            entry(3, 4, 3, 20, 40),
        ]);

        assert_eq!(
            store
                .find_for_item_like_cpp(4, 0, 2, 30, 0)
                .map(|entry| entry.id),
            Some(1)
        );
        assert_eq!(
            store
                .find_for_item_like_cpp(4, 1, 2, 30, 0)
                .map(|entry| entry.id),
            Some(2)
        );
        assert!(store.find_for_item_like_cpp(4, 0, 2, 10, 0).is_none());
        assert!(store.find_for_item_like_cpp(2, 0, 2, 30, 0).is_none());
        assert_eq!(
            store
                .find_for_item_like_cpp(4, 0, 3, 30, 0)
                .map(|entry| entry.id),
            Some(3)
        );
    }
}
