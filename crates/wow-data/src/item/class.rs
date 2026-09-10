// Copyright (c) 2026 alseif0x
// RustyCore - WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 - https://www.gnu.org/licenses/gpl-3.0.html

//! ItemClass.db2 reader.

use std::collections::HashMap;
use std::path::Path;

use anyhow::{Context, Result};
use tracing::info;

use crate::wdc4::Wdc4Reader;

/// C++ `ItemClassEntry`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ItemClassEntry {
    pub id: u32,
    pub class_id: i8,
    pub price_modifier: f32,
    pub flags: u8,
}

/// In-memory store for `ItemClass.db2`.
pub struct ItemClassStore {
    entries: HashMap<u32, ItemClassEntry>,
    by_old_enum: HashMap<u32, ItemClassEntry>,
}

impl ItemClassStore {
    pub fn from_entries(entries: impl IntoIterator<Item = ItemClassEntry>) -> Self {
        let entries: HashMap<_, _> = entries.into_iter().map(|entry| (entry.id, entry)).collect();
        let by_old_enum = entries
            .values()
            .filter_map(|entry| {
                u32::try_from(entry.class_id)
                    .ok()
                    .map(|class| (class, *entry))
            })
            .collect();

        Self {
            entries,
            by_old_enum,
        }
    }

    /// Load ItemClass.db2 from `{data_dir}/dbc/{locale}/ItemClass.db2`.
    ///
    /// C++ refs:
    /// - `DB2Structure.h::ItemClassEntry`
    /// - `DB2LoadInfo.h::ItemClassLoadInfo`
    /// - `DB2Manager::GetItemClassByOldEnum`
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        let path = Path::new(data_dir)
            .join("dbc")
            .join(locale)
            .join("ItemClass.db2");

        let reader = Wdc4Reader::open(&path)
            .with_context(|| format!("failed to open {}", path.display()))?;

        let mut entries = HashMap::with_capacity(reader.total_count());
        let mut by_old_enum = HashMap::new();
        for (id, idx) in reader.iter_records() {
            let entry = ItemClassEntry {
                id,
                class_id: reader.get_field_i8(idx, 1),
                price_modifier: f32::from_bits(reader.get_field_u32(idx, 2)),
                flags: reader.get_field_u8(idx, 3),
            };
            if let Ok(class_id) = u32::try_from(entry.class_id) {
                by_old_enum.insert(class_id, entry);
            }
            entries.insert(id, entry);
        }

        info!(
            "Loaded {} item classes from {}",
            entries.len(),
            path.display()
        );
        Ok(Self {
            entries,
            by_old_enum,
        })
    }

    pub fn get(&self, id: u32) -> Option<&ItemClassEntry> {
        self.entries.get(&id)
    }

    /// C++ `sDB2Manager.GetItemClassByOldEnum(itemClass)`.
    pub fn get_by_old_enum(&self, item_class: u32) -> Option<&ItemClassEntry> {
        self.by_old_enum.get(&item_class)
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
    fn item_class_store_indexes_by_id_and_old_enum_like_cpp_manager() {
        let store = ItemClassStore::from_entries([
            ItemClassEntry {
                id: 1,
                class_id: 4,
                price_modifier: 0.25,
                flags: 0,
            },
            ItemClassEntry {
                id: 2,
                class_id: -1,
                price_modifier: 1.0,
                flags: 0,
            },
        ]);

        assert_eq!(store.get(1).unwrap().price_modifier, 0.25);
        assert_eq!(store.get_by_old_enum(4).unwrap().id, 1);
        assert!(store.get_by_old_enum(u32::MAX).is_none());
    }
}
