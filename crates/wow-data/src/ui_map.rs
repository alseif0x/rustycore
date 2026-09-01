// Copyright (c) 2026 alseif0x
// RustyCore - WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 - https://www.gnu.org/licenses/gpl-3.0.html

//! Minimal UiMap DB2 readers needed by C++ `DB2Manager` lookups.

use std::collections::{HashMap, HashSet};
use std::path::Path;

use anyhow::{Context, Result};
use tracing::info;

use crate::wdc4::Wdc4Reader;

/// C++ `UiMapXMapArtEntry`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UiMapXMapArtEntry {
    pub id: u32,
    pub phase_id: i32,
    pub ui_map_art_id: i32,
    pub ui_map_id: u32,
}

/// In-memory subset of `UiMapXMapArt.db2`.
///
/// C++ `DB2Manager::LoadStores` builds `_uiMapPhases` by inserting every
/// non-zero `UiMapXMapArtEntry::PhaseID`. `ObjectMgr::LoadTerrainWorldMaps`
/// then calls `DB2Manager::IsUiMapPhase` to validate `terrain_worldmap`.
pub struct UiMapXMapArtStore {
    entries: HashMap<u32, UiMapXMapArtEntry>,
    ui_map_phases: HashSet<u32>,
}

impl UiMapXMapArtStore {
    pub fn from_entries(entries: impl IntoIterator<Item = UiMapXMapArtEntry>) -> Self {
        let entries: HashMap<_, _> = entries.into_iter().map(|entry| (entry.id, entry)).collect();
        let ui_map_phases = entries
            .values()
            .filter_map(|entry| {
                if entry.phase_id == 0 {
                    None
                } else {
                    u32::try_from(entry.phase_id).ok()
                }
            })
            .collect();

        Self {
            entries,
            ui_map_phases,
        }
    }

    /// Load UiMapXMapArt.db2 from `{data_dir}/dbc/{locale}/UiMapXMapArt.db2`.
    ///
    /// C++ refs:
    /// - `DB2Structure.h::UiMapXMapArtEntry`
    /// - `DB2LoadInfo.h::UiMapXMapArtLoadInfo`
    /// - `DB2Stores.cpp::_uiMapPhases`
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        let path = Path::new(data_dir)
            .join("dbc")
            .join(locale)
            .join("UiMapXMapArt.db2");

        let reader = Wdc4Reader::open(&path)
            .with_context(|| format!("failed to open {}", path.display()))?;

        let mut entries = HashMap::with_capacity(reader.total_count());
        let mut ui_map_phases = HashSet::new();
        for (id, idx) in reader.iter_records() {
            let entry = UiMapXMapArtEntry {
                id,
                // WDC4 record ids supply C++ field 0 (`ID`), so PhaseID is
                // field 0 in this reader. C++ `UiMapID` is DB2Meta parent
                // index 2 and is exposed by this reader as the relationship id.
                phase_id: reader.get_field_i32(idx, 0),
                ui_map_art_id: reader.get_field_i32(idx, 1),
                ui_map_id: reader.get_relationship_id(idx).unwrap_or(0),
            };
            if let Ok(phase_id) = u32::try_from(entry.phase_id) {
                if phase_id != 0 {
                    ui_map_phases.insert(phase_id);
                }
            }
            entries.insert(id, entry);
        }

        info!(
            "Loaded {} UiMapXMapArt rows and {} UI map phases from {}",
            entries.len(),
            ui_map_phases.len(),
            path.display()
        );
        Ok(Self {
            entries,
            ui_map_phases,
        })
    }

    pub fn apply_hotfix_rows_like_cpp(
        &mut self,
        rows: impl IntoIterator<Item = UiMapXMapArtEntry>,
    ) -> usize {
        let mut count = 0usize;
        for entry in rows {
            self.entries.insert(entry.id, entry);
            count += 1;
        }
        self.rebuild_ui_map_phases();
        count
    }

    fn rebuild_ui_map_phases(&mut self) {
        self.ui_map_phases = self
            .entries
            .values()
            .filter_map(|entry| {
                if entry.phase_id == 0 {
                    None
                } else {
                    u32::try_from(entry.phase_id).ok()
                }
            })
            .collect();
    }

    pub fn get(&self, id: u32) -> Option<&UiMapXMapArtEntry> {
        self.entries.get(&id)
    }

    /// C++ `DB2Manager::IsUiMapPhase`.
    pub fn is_ui_map_phase(&self, phase_id: u32) -> bool {
        self.ui_map_phases.contains(&phase_id)
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
    fn ui_map_x_map_art_store_indexes_nonzero_phase_ids_like_cpp_manager() {
        let store = UiMapXMapArtStore::from_entries([
            UiMapXMapArtEntry {
                id: 1,
                phase_id: 42,
                ui_map_art_id: 10,
                ui_map_id: 100,
            },
            UiMapXMapArtEntry {
                id: 2,
                phase_id: 0,
                ui_map_art_id: 11,
                ui_map_id: 101,
            },
            UiMapXMapArtEntry {
                id: 3,
                phase_id: -7,
                ui_map_art_id: 12,
                ui_map_id: 102,
            },
        ]);

        assert!(store.is_ui_map_phase(42));
        assert!(!store.is_ui_map_phase(0));
        assert!(!store.is_ui_map_phase(7));
        assert_eq!(store.get(1).unwrap().ui_map_id, 100);
    }

    #[test]
    fn load_ui_map_x_map_art_db2_when_fixture_exists() {
        let data_dir = "/home/server/woltk-server-core/Data";
        let locale = "esES";
        let path = Path::new(data_dir)
            .join("dbc")
            .join(locale)
            .join("UiMapXMapArt.db2");
        if !path.exists() {
            eprintln!(
                "Skipping test: UiMapXMapArt.db2 not found at {}",
                path.display()
            );
            return;
        }

        let store =
            UiMapXMapArtStore::load(data_dir, locale).expect("failed to load UiMapXMapArt.db2");
        assert!(!store.is_empty());
    }
}
