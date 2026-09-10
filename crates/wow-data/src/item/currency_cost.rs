// Copyright (c) 2026 alseif0x
// RustyCore - WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 - https://www.gnu.org/licenses/gpl-3.0.html

//! ItemCurrencyCost.db2 reader.

use std::collections::{HashMap, HashSet};
use std::path::Path;

use anyhow::{Context, Result};
use tracing::info;

use crate::wdc4::Wdc4Reader;

/// C++ `ItemCurrencyCostEntry`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ItemCurrencyCostEntry {
    pub id: u32,
    pub item_id: i32,
}

/// In-memory store for `ItemCurrencyCost.db2`.
pub struct ItemCurrencyCostStore {
    entries: HashMap<u32, ItemCurrencyCostEntry>,
    items_with_currency_cost: HashSet<u32>,
}

impl ItemCurrencyCostStore {
    pub fn from_entries(entries: impl IntoIterator<Item = ItemCurrencyCostEntry>) -> Self {
        let entries: HashMap<_, _> = entries.into_iter().map(|entry| (entry.id, entry)).collect();
        let items_with_currency_cost = entries
            .values()
            .filter_map(|entry| u32::try_from(entry.item_id).ok())
            .collect();

        Self {
            entries,
            items_with_currency_cost,
        }
    }

    /// Load ItemCurrencyCost.db2 from `{data_dir}/dbc/{locale}/ItemCurrencyCost.db2`.
    ///
    /// C++ refs:
    /// - `DB2Structure.h::ItemCurrencyCostEntry`
    /// - `DB2LoadInfo.h::ItemCurrencyCostLoadInfo`
    /// - `DB2Manager::HasItemCurrencyCost`
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        let path = Path::new(data_dir)
            .join("dbc")
            .join(locale)
            .join("ItemCurrencyCost.db2");

        let reader = Wdc4Reader::open(&path)
            .with_context(|| format!("failed to open {}", path.display()))?;

        let mut entries = HashMap::with_capacity(reader.total_count());
        let mut items_with_currency_cost = HashSet::new();
        for (id, idx) in reader.iter_records() {
            let entry = ItemCurrencyCostEntry {
                id,
                item_id: reader.get_field_i32(idx, 0),
            };
            if let Ok(item_id) = u32::try_from(entry.item_id) {
                items_with_currency_cost.insert(item_id);
            }
            entries.insert(id, entry);
        }

        info!(
            "Loaded {} item currency costs from {}",
            entries.len(),
            path.display()
        );
        Ok(Self {
            entries,
            items_with_currency_cost,
        })
    }

    pub fn get(&self, id: u32) -> Option<&ItemCurrencyCostEntry> {
        self.entries.get(&id)
    }

    /// C++ `DB2Manager::HasItemCurrencyCost(itemId)`.
    pub fn has_item_currency_cost(&self, item_id: u32) -> bool {
        self.items_with_currency_cost.contains(&item_id)
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
    fn item_currency_cost_store_indexes_entries_and_items_like_cpp_manager() {
        let store = ItemCurrencyCostStore::from_entries([
            ItemCurrencyCostEntry {
                id: 10,
                item_id: 1000,
            },
            ItemCurrencyCostEntry {
                id: 11,
                item_id: -1,
            },
        ]);

        assert_eq!(store.get(10).unwrap().item_id, 1000);
        assert!(store.has_item_currency_cost(1000));
        assert!(!store.has_item_currency_cost(11));
        assert!(!store.has_item_currency_cost(u32::MAX));
    }
}
