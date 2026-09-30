// Copyright (c) 2026 alseif0x
// RustyCore - WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 - https://www.gnu.org/licenses/gpl-3.0.html

//! CurrencyTypes.db2 reader.

use std::collections::HashMap;
use std::path::Path;

use anyhow::{Context, Result};
use tracing::info;
use wow_constants::{CurrencyTypesFlags, CurrencyTypesFlagsB};

use crate::wdc4::Wdc4Reader;

pub use wow_data_model::currency::CurrencyTypesEntry;

/// In-memory store for `CurrencyTypes.db2`.
pub struct CurrencyTypesStore {
    entries: HashMap<u32, CurrencyTypesEntry>,
}

impl CurrencyTypesStore {
    pub fn from_entries(entries: impl IntoIterator<Item = CurrencyTypesEntry>) -> Self {
        Self {
            entries: entries.into_iter().map(|entry| (entry.id, entry)).collect(),
        }
    }

    /// Load CurrencyTypes.db2 from `{data_dir}/dbc/{locale}/CurrencyTypes.db2`.
    ///
    /// C++ refs:
    /// - `DB2Structure.h::CurrencyTypesEntry`
    /// - `DB2LoadInfo.h::CurrencyTypesLoadInfo`
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        let path = Path::new(data_dir)
            .join("dbc")
            .join(locale)
            .join("CurrencyTypes.db2");

        let reader = Wdc4Reader::open(&path)
            .with_context(|| format!("failed to open {}", path.display()))?;

        let mut entries = HashMap::with_capacity(reader.total_count());
        for (id, idx) in reader.iter_records() {
            let record = CurrencyTypesEntry {
                id,
                category_id: reader.get_field_u8(idx, 2),
                inventory_icon_file_id: reader.get_field_i32(idx, 3),
                spell_weight: reader.get_field_u32(idx, 4),
                spell_category: reader.get_field_u8(idx, 5),
                max_qty: reader.get_field_u32(idx, 6),
                max_earnable_per_week: reader.get_field_u32(idx, 7),
                quality: reader.get_field_i8(idx, 8),
                faction_id: reader.get_field_i32(idx, 9),
                award_condition_id: reader.get_field_i32(idx, 10),
                flags: CurrencyTypesFlags::from_bits_retain(
                    reader.get_array_element(idx, 11, 0, 32),
                ),
                flags_b: CurrencyTypesFlagsB::from_bits_retain(
                    reader.get_array_element(idx, 11, 1, 32),
                ),
            };
            entries.insert(id, record);
        }

        info!(
            "Loaded {} currencies from {}",
            entries.len(),
            path.display()
        );
        Ok(Self { entries })
    }

    pub fn get(&self, id: u32) -> Option<&CurrencyTypesEntry> {
        self.entries.get(&id)
    }

    pub fn has_record(&self, id: u32) -> bool {
        self.entries.contains_key(&id)
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
    fn load_currency_types_store() {
        let data_dir = "/home/server/woltk-server-core/Data";
        let locale = "esES";
        let path = Path::new(data_dir)
            .join("dbc")
            .join(locale)
            .join("CurrencyTypes.db2");
        if !path.exists() {
            eprintln!(
                "Skipping test: CurrencyTypes.db2 not found at {}",
                path.display()
            );
            return;
        }

        let store =
            CurrencyTypesStore::load(data_dir, locale).expect("failed to load CurrencyTypesStore");
        assert!(!store.is_empty());
        let known_id = *store
            .entries
            .keys()
            .next()
            .expect("loaded CurrencyTypes.db2 should contain at least one ID");
        assert!(store.has_record(known_id));
        assert_eq!(store.get(known_id).map(|entry| entry.id), Some(known_id));
    }
}
