// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! ItemAppearance.db2 reader.

use std::collections::HashMap;
use std::path::Path;

use anyhow::{Context, Result};
use tracing::info;

use crate::wdc4::Wdc4Reader;

/// C++ `ItemAppearanceEntry`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ItemAppearanceEntry {
    pub id: u32,
    pub display_type: u8,
    pub item_display_info_id: i32,
    pub default_icon_file_data_id: i32,
    pub ui_order: i32,
}

/// In-memory store for `ItemAppearance.db2`.
pub struct ItemAppearanceStore {
    entries: HashMap<u32, ItemAppearanceEntry>,
}

impl ItemAppearanceStore {
    pub fn from_entries(entries: impl IntoIterator<Item = ItemAppearanceEntry>) -> Self {
        Self {
            entries: entries.into_iter().map(|entry| (entry.id, entry)).collect(),
        }
    }

    /// Load ItemAppearance.db2 from `{data_dir}/dbc/{locale}/ItemAppearance.db2`.
    ///
    /// C++ refs:
    /// - `DB2Structure.h::ItemAppearanceEntry`
    /// - `DB2LoadInfo.h::ItemAppearanceLoadInfo`
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        let path = Path::new(data_dir)
            .join("dbc")
            .join(locale)
            .join("ItemAppearance.db2");

        let reader = Wdc4Reader::open(&path)
            .with_context(|| format!("failed to open {}", path.display()))?;

        let mut entries = HashMap::with_capacity(reader.total_count());
        for (id, idx) in reader.iter_records() {
            let record = ItemAppearanceEntry {
                id,
                display_type: reader.get_field_u8(idx, 0),
                item_display_info_id: reader.get_field_i32(idx, 1),
                default_icon_file_data_id: reader.get_field_i32(idx, 2),
                ui_order: reader.get_field_i32(idx, 3),
            };
            entries.insert(id, record);
        }

        info!(
            "Loaded {} item appearances from {}",
            entries.len(),
            path.display()
        );
        Ok(Self { entries })
    }

    pub fn get(&self, id: u32) -> Option<&ItemAppearanceEntry> {
        self.entries.get(&id)
    }

    pub fn item_display_info_id(&self, id: u32) -> Option<u32> {
        self.get(id)
            .and_then(|entry| u32::try_from(entry.item_display_info_id).ok())
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
    fn load_item_appearance_store() {
        let data_dir = "/home/server/woltk-server-core/Data";
        let locale = "esES";
        let path = Path::new(data_dir)
            .join("dbc")
            .join(locale)
            .join("ItemAppearance.db2");
        if !path.exists() {
            eprintln!(
                "Skipping test: ItemAppearance.db2 not found at {}",
                path.display()
            );
            return;
        }

        let store = ItemAppearanceStore::load(data_dir, locale)
            .expect("failed to load ItemAppearanceStore");
        assert!(!store.is_empty());
        assert!(
            store
                .entries
                .values()
                .any(|entry| entry.item_display_info_id > 0)
        );
    }

    #[test]
    fn item_display_info_id_rejects_negative_values() {
        let store = ItemAppearanceStore::from_entries([ItemAppearanceEntry {
            id: 1,
            display_type: 0,
            item_display_info_id: -1,
            default_icon_file_data_id: 0,
            ui_order: 0,
        }]);

        assert_eq!(store.item_display_info_id(1), None);
        assert_eq!(store.item_display_info_id(2), None);
    }
}
