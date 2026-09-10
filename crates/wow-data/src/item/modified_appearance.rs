// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! ItemModifiedAppearance.db2 reader.

use std::collections::HashMap;
use std::path::Path;

use anyhow::{Context, Result};
use tracing::info;

use crate::wdc4::Wdc4Reader;

/// C++ `ItemModifiedAppearanceEntry`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ItemModifiedAppearanceEntry {
    pub id: u32,
    pub item_id: i32,
    pub item_appearance_modifier_id: i32,
    pub item_appearance_id: i32,
    pub order_index: i32,
    pub transmog_source_type_enum: i32,
}

/// In-memory store for `ItemModifiedAppearance.db2`.
pub struct ItemModifiedAppearanceStore {
    entries: HashMap<u32, ItemModifiedAppearanceEntry>,
    by_item: HashMap<u32, u32>,
}

impl ItemModifiedAppearanceStore {
    pub fn from_entries(entries: impl IntoIterator<Item = ItemModifiedAppearanceEntry>) -> Self {
        let mut store = Self {
            entries: HashMap::new(),
            by_item: HashMap::new(),
        };
        for entry in entries {
            store.insert(entry);
        }
        store
    }

    /// Load ItemModifiedAppearance.db2 from `{data_dir}/dbc/{locale}/ItemModifiedAppearance.db2`.
    ///
    /// C++ refs:
    /// - `DB2Structure.h::ItemModifiedAppearanceEntry`
    /// - `DB2LoadInfo.h::ItemModifiedAppearanceLoadInfo`
    /// - `DB2Manager::GetItemModifiedAppearance`
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        let path = Path::new(data_dir)
            .join("dbc")
            .join(locale)
            .join("ItemModifiedAppearance.db2");

        let reader = Wdc4Reader::open(&path)
            .with_context(|| format!("failed to open {}", path.display()))?;

        let mut store = Self {
            entries: HashMap::with_capacity(reader.total_count()),
            by_item: HashMap::with_capacity(reader.total_count()),
        };
        for (id, idx) in reader.iter_records() {
            let record = ItemModifiedAppearanceEntry {
                id,
                item_id: reader.get_field_i32(idx, 1),
                item_appearance_modifier_id: reader.get_field_i32(idx, 2),
                item_appearance_id: reader.get_field_i32(idx, 3),
                order_index: reader.get_field_i32(idx, 4),
                transmog_source_type_enum: reader.get_field_i32(idx, 5),
            };
            store.insert(record);
        }

        info!(
            "Loaded {} item modified appearances from {}",
            store.entries.len(),
            path.display()
        );
        Ok(store)
    }

    fn insert(&mut self, entry: ItemModifiedAppearanceEntry) {
        if let (Ok(item_id), Ok(appearance_mod_id)) = (
            u32::try_from(entry.item_id),
            u32::try_from(entry.item_appearance_modifier_id),
        ) {
            self.by_item
                .insert(item_appearance_key(item_id, appearance_mod_id), entry.id);
        }
        self.entries.insert(entry.id, entry);
    }

    pub fn get(&self, id: u32) -> Option<&ItemModifiedAppearanceEntry> {
        self.entries.get(&id)
    }

    /// C++ `DB2Manager::GetItemModifiedAppearance`.
    pub fn get_for_item(
        &self,
        item_id: u32,
        appearance_mod_id: u32,
    ) -> Option<&ItemModifiedAppearanceEntry> {
        self.by_item
            .get(&item_appearance_key(item_id, appearance_mod_id))
            .and_then(|id| self.get(*id))
            .or_else(|| {
                (appearance_mod_id != 0)
                    .then(|| self.by_item.get(&item_appearance_key(item_id, 0)))
                    .flatten()
                    .and_then(|id| self.get(*id))
            })
    }

    /// C++ `DB2Manager::GetDefaultItemModifiedAppearance`.
    pub fn get_default_for_item(&self, item_id: u32) -> Option<&ItemModifiedAppearanceEntry> {
        self.by_item
            .get(&item_appearance_key(item_id, 0))
            .and_then(|id| self.get(*id))
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

const fn item_appearance_key(item_id: u32, appearance_mod_id: u32) -> u32 {
    item_id | (appearance_mod_id << 24)
}

#[cfg(test)]
mod tests {
    use super::*;

    const fn appearance(
        id: u32,
        item_id: i32,
        item_appearance_modifier_id: i32,
    ) -> ItemModifiedAppearanceEntry {
        ItemModifiedAppearanceEntry {
            id,
            item_id,
            item_appearance_modifier_id,
            item_appearance_id: 1000 + id as i32,
            order_index: 0,
            transmog_source_type_enum: 0,
        }
    }

    #[test]
    fn load_item_modified_appearance_store() {
        let data_dir = "/home/server/woltk-server-core/Data";
        let locale = "esES";
        let path = Path::new(data_dir)
            .join("dbc")
            .join(locale)
            .join("ItemModifiedAppearance.db2");
        if !path.exists() {
            eprintln!(
                "Skipping test: ItemModifiedAppearance.db2 not found at {}",
                path.display()
            );
            return;
        }

        let store = ItemModifiedAppearanceStore::load(data_dir, locale)
            .expect("failed to load ItemModifiedAppearanceStore");
        assert!(!store.is_empty());
        assert!(store.entries.values().any(|entry| entry.item_id > 0));
    }

    #[test]
    fn get_for_item_matches_cpp_fallback_key() {
        let store = ItemModifiedAppearanceStore::from_entries([
            appearance(10, 100, 0),
            appearance(11, 100, 2),
            appearance(12, 101, 0),
        ]);

        assert_eq!(store.get_for_item(100, 2).unwrap().id, 11);
        assert_eq!(store.get_for_item(100, 9).unwrap().id, 10);
        assert_eq!(store.get_default_for_item(101).unwrap().id, 12);
        assert!(store.get_for_item(102, 0).is_none());
    }
}
