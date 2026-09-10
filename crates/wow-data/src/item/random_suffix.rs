// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! ItemRandomSuffix.db2 reader.

use std::collections::HashMap;
use std::path::Path;

use anyhow::{Context, Result};
use tracing::info;

use crate::wdc4::Wdc4Reader;

pub const ITEM_RANDOM_SUFFIX_EFFECTS: usize = 5;

/// C++ `ItemRandomSuffixEntry`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ItemRandomSuffixEntry {
    pub id: u32,
    pub enchantments: [u16; ITEM_RANDOM_SUFFIX_EFFECTS],
    pub allocation_pct: [u16; ITEM_RANDOM_SUFFIX_EFFECTS],
}

/// In-memory store for `ItemRandomSuffix.db2`.
pub struct ItemRandomSuffixStore {
    entries: HashMap<u32, ItemRandomSuffixEntry>,
}

impl ItemRandomSuffixStore {
    pub fn from_entries(entries: impl IntoIterator<Item = ItemRandomSuffixEntry>) -> Self {
        Self {
            entries: entries.into_iter().map(|entry| (entry.id, entry)).collect(),
        }
    }

    /// Load ItemRandomSuffix.db2 from `{data_dir}/dbc/{locale}/ItemRandomSuffix.db2`.
    ///
    /// C++ refs:
    /// - `DB2Structure.h::ItemRandomSuffixEntry`
    /// - `DB2LoadInfo.h::ItemRandomSuffixLoadInfo`
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        let path = Path::new(data_dir)
            .join("dbc")
            .join(locale)
            .join("ItemRandomSuffix.db2");

        let reader = Wdc4Reader::open(&path)
            .with_context(|| format!("failed to open {}", path.display()))?;

        let mut entries = HashMap::with_capacity(reader.total_count());
        for (id, idx) in reader.iter_records() {
            let record = ItemRandomSuffixEntry {
                id,
                enchantments: [
                    reader.get_array_element(idx, 1, 0, 16) as u16,
                    reader.get_array_element(idx, 1, 1, 16) as u16,
                    reader.get_array_element(idx, 1, 2, 16) as u16,
                    reader.get_array_element(idx, 1, 3, 16) as u16,
                    reader.get_array_element(idx, 1, 4, 16) as u16,
                ],
                allocation_pct: [
                    reader.get_array_element(idx, 2, 0, 16) as u16,
                    reader.get_array_element(idx, 2, 1, 16) as u16,
                    reader.get_array_element(idx, 2, 2, 16) as u16,
                    reader.get_array_element(idx, 2, 3, 16) as u16,
                    reader.get_array_element(idx, 2, 4, 16) as u16,
                ],
            };
            entries.insert(id, record);
        }

        info!(
            "Loaded {} item random suffixes from {}",
            entries.len(),
            path.display()
        );
        Ok(Self { entries })
    }

    pub fn get(&self, id: u32) -> Option<&ItemRandomSuffixEntry> {
        self.entries.get(&id)
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
    fn load_item_random_suffix_store() {
        let data_dir = "/home/server/woltk-server-core/Data";
        let locale = "esES";
        let path = Path::new(data_dir)
            .join("dbc")
            .join(locale)
            .join("ItemRandomSuffix.db2");
        if !path.exists() {
            eprintln!(
                "Skipping test: ItemRandomSuffix.db2 not found at {}",
                path.display()
            );
            return;
        }

        let store = ItemRandomSuffixStore::load(data_dir, locale)
            .expect("failed to load ItemRandomSuffixStore");
        assert!(!store.is_empty());
        assert!(
            store
                .entries
                .values()
                .any(|entry| entry.enchantments.iter().any(|value| *value != 0))
        );
    }
}
