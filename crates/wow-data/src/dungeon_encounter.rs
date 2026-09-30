// Copyright (c) 2026 alseif0x
// RustyCore - WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 - https://www.gnu.org/licenses/gpl-3.0.html

//! DungeonEncounter.db2 reader.

use std::collections::HashMap;
use std::path::Path;

use anyhow::{Context, Result};
use tracing::info;

use crate::wdc4::Wdc4Reader;

pub use wow_data_model::dungeon_encounter::DungeonEncounterEntry;

/// In-memory store for `DungeonEncounter.db2`.
pub struct DungeonEncounterStore {
    entries: HashMap<u32, DungeonEncounterEntry>,
}

impl DungeonEncounterStore {
    pub fn from_entries(entries: impl IntoIterator<Item = DungeonEncounterEntry>) -> Self {
        Self {
            entries: entries.into_iter().map(|entry| (entry.id, entry)).collect(),
        }
    }

    /// Load DungeonEncounter.db2 from `{data_dir}/dbc/{locale}/DungeonEncounter.db2`.
    ///
    /// C++ refs:
    /// - `DB2Structure.h::DungeonEncounterEntry`
    /// - `DB2LoadInfo.h::DungeonEncounterLoadInfo`
    /// - `DB2Stores.cpp::sDungeonEncounterStore`
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        let path = Path::new(data_dir)
            .join("dbc")
            .join(locale)
            .join("DungeonEncounter.db2");

        let reader = Wdc4Reader::open(&path)
            .with_context(|| format!("failed to open {}", path.display()))?;

        let mut entries = HashMap::with_capacity(reader.total_count());
        for (id, idx) in reader.iter_records() {
            let entry = DungeonEncounterEntry {
                id,
                // Field 0 is localized Name. C++ field 1 is ID, supplied by
                // WDC4 record id in this reader like other Rust DB2 stores.
                map_id: reader.get_field_i16(idx, 2),
                difficulty_id: reader.get_field_i32(idx, 3),
                order_index: reader.get_field_i32(idx, 4),
                bit: reader.get_field_i8(idx, 5),
                flags: reader.get_field_i32(idx, 6),
                faction: reader.get_field_i32(idx, 7),
            };
            entries.insert(id, entry);
        }

        info!(
            "Loaded {} dungeon encounters from {}",
            entries.len(),
            path.display()
        );
        Ok(Self { entries })
    }

    pub fn get(&self, id: u32) -> Option<&DungeonEncounterEntry> {
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
    fn dungeon_encounter_store_indexes_by_id_like_cpp_store() {
        let store = DungeonEncounterStore::from_entries([DungeonEncounterEntry {
            id: 615,
            map_id: 631,
            difficulty_id: 4,
            order_index: 0,
            bit: 0,
            flags: 0,
            faction: -1,
        }]);

        let encounter = store.get(615).unwrap();
        assert_eq!(encounter.map_id, 631);
        assert_eq!(encounter.difficulty_id, 4);
        assert!(store.get(616).is_none());
    }

    #[test]
    fn load_dungeon_encounter_db2_when_fixture_exists() {
        let data_dir = "/home/server/woltk-server-core/Data";
        let locale = "esES";
        let path = Path::new(data_dir)
            .join("dbc")
            .join(locale)
            .join("DungeonEncounter.db2");
        if !path.exists() {
            eprintln!(
                "Skipping test: DungeonEncounter.db2 not found at {}",
                path.display()
            );
            return;
        }

        let store =
            DungeonEncounterStore::load(data_dir, locale).expect("failed to load encounters");
        assert!(!store.is_empty());

        if let Some(encounter) = store.get(615) {
            assert!(encounter.map_id > 0);
            assert!(encounter.difficulty_id >= 0);
        }
    }
}
