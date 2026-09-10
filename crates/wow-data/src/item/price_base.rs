// Copyright (c) 2026 alseif0x
// RustyCore - WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 - https://www.gnu.org/licenses/gpl-3.0.html

//! ItemPriceBase.db2 reader used by C++ `Item::GetBuyPrice`.

use std::collections::HashMap;
use std::path::Path;

use anyhow::{Context, Result};
use tracing::info;

use crate::wdc4::Wdc4Reader;

/// C++ `ItemPriceBaseEntry`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ItemPriceBaseEntry {
    pub id: u32,
    pub item_level: u16,
    pub armor: f32,
    pub weapon: f32,
}

pub struct ItemPriceBaseStore {
    entries: HashMap<u32, ItemPriceBaseEntry>,
}

impl ItemPriceBaseStore {
    pub fn from_entries(entries: impl IntoIterator<Item = ItemPriceBaseEntry>) -> Self {
        Self {
            entries: entries.into_iter().map(|entry| (entry.id, entry)).collect(),
        }
    }

    /// Load ItemPriceBase.db2 from `{data_dir}/dbc/{locale}/ItemPriceBase.db2`.
    ///
    /// C++ refs:
    /// - `DB2Structure.h::ItemPriceBaseEntry`
    /// - `DB2LoadInfo.h::ItemPriceBaseLoadInfo`
    /// - `Item::GetBuyPrice`
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        let path = Path::new(data_dir)
            .join("dbc")
            .join(locale)
            .join("ItemPriceBase.db2");

        let reader = Wdc4Reader::open(&path)
            .with_context(|| format!("failed to open {}", path.display()))?;
        let base = if reader.field_count() >= 4 { 1 } else { 0 };
        let mut entries = HashMap::with_capacity(reader.total_count());

        for (id, idx) in reader.iter_records() {
            entries.insert(
                id,
                ItemPriceBaseEntry {
                    id,
                    item_level: reader.get_field_u16(idx, base),
                    armor: f32::from_bits(reader.get_field_u32(idx, base + 1)),
                    weapon: f32::from_bits(reader.get_field_u32(idx, base + 2)),
                },
            );
        }

        info!(
            "Loaded {} item price base rows from {}",
            entries.len(),
            path.display()
        );
        Ok(Self { entries })
    }

    /// C++ `sItemPriceBaseStore.LookupEntry(itemLevel)`.
    pub fn get(&self, item_level: u32) -> Option<&ItemPriceBaseEntry> {
        self.entries.get(&item_level)
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
    fn item_price_base_store_indexes_by_item_level_like_cpp_lookup() {
        let store = ItemPriceBaseStore::from_entries([ItemPriceBaseEntry {
            id: 57,
            item_level: 57,
            armor: 12.5,
            weapon: 44.25,
        }]);

        let entry = store.get(57).unwrap();
        assert_eq!(entry.item_level, 57);
        assert_eq!(entry.armor, 12.5);
        assert_eq!(entry.weapon, 44.25);
        assert!(store.get(58).is_none());
    }
}
