// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! ItemRandomProperties.db2 reader.

use std::collections::HashMap;
use std::path::Path;

use anyhow::{Context, Result};
use tracing::info;

use crate::wdc4::Wdc4Reader;

pub const ITEM_RANDOM_PROPERTIES_EFFECTS: usize = 5;

/// C++ `ItemRandomPropertiesEntry`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ItemRandomPropertiesEntry {
    pub id: u32,
    pub enchantments: [u16; ITEM_RANDOM_PROPERTIES_EFFECTS],
}

/// In-memory store for `ItemRandomProperties.db2`.
pub struct ItemRandomPropertiesStore {
    entries: HashMap<u32, ItemRandomPropertiesEntry>,
}

impl ItemRandomPropertiesStore {
    pub fn from_entries(entries: impl IntoIterator<Item = ItemRandomPropertiesEntry>) -> Self {
        Self {
            entries: entries.into_iter().map(|entry| (entry.id, entry)).collect(),
        }
    }

    /// Load ItemRandomProperties.db2 from `{data_dir}/dbc/{locale}/ItemRandomProperties.db2`.
    ///
    /// C++ refs:
    /// - `DB2Structure.h::ItemRandomPropertiesEntry`
    /// - `DB2LoadInfo.h::ItemRandomPropertiesLoadInfo`
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        let path = Path::new(data_dir)
            .join("dbc")
            .join(locale)
            .join("ItemRandomProperties.db2");

        let reader = Wdc4Reader::open(&path)
            .with_context(|| format!("failed to open {}", path.display()))?;

        let mut entries = HashMap::with_capacity(reader.total_count());
        for (id, idx) in reader.iter_records() {
            let record = ItemRandomPropertiesEntry {
                id,
                enchantments: [
                    reader.get_array_element(idx, 1, 0, 16) as u16,
                    reader.get_array_element(idx, 1, 1, 16) as u16,
                    reader.get_array_element(idx, 1, 2, 16) as u16,
                    reader.get_array_element(idx, 1, 3, 16) as u16,
                    reader.get_array_element(idx, 1, 4, 16) as u16,
                ],
            };
            entries.insert(id, record);
        }

        info!(
            "Loaded {} item random properties from {}",
            entries.len(),
            path.display()
        );
        Ok(Self { entries })
    }

    pub fn get(&self, id: u32) -> Option<&ItemRandomPropertiesEntry> {
        self.entries.get(&id)
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}
