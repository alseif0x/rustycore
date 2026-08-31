// Copyright (c) 2026 alseif0x
// RustyCore - WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 - https://www.gnu.org/licenses/gpl-3.0.html

//! Phase.db2 and PhaseXPhaseGroup.db2 readers.

use std::collections::HashMap;
use std::path::Path;

use anyhow::{Context, Result};
use tracing::info;

use crate::wdc4::Wdc4Reader;

pub const PHASE_ENTRY_FLAG_COSMETIC: u16 = 0x0010;
pub const PHASE_ENTRY_FLAG_PERSONAL: u16 = 0x0020;

/// C++ `PhaseEntry`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PhaseEntry {
    pub id: u32,
    pub flags: u16,
}

/// C++ `PhaseXPhaseGroupEntry`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PhaseXPhaseGroupEntry {
    pub id: u32,
    pub phase_id: u16,
    pub phase_group_id: u32,
}

pub struct PhaseStore {
    entries: HashMap<u32, PhaseEntry>,
}

impl PhaseStore {
    pub fn from_entries(entries: impl IntoIterator<Item = PhaseEntry>) -> Self {
        Self {
            entries: entries.into_iter().map(|entry| (entry.id, entry)).collect(),
        }
    }

    /// Load Phase.db2 from `{data_dir}/dbc/{locale}/Phase.db2`.
    ///
    /// C++ refs:
    /// - `DB2Structure.h::PhaseEntry`
    /// - `DB2LoadInfo.h::PhaseLoadInfo`
    /// - `PhasingHandler.cpp::GetPhaseFlags`
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        let path = Path::new(data_dir)
            .join("dbc")
            .join(locale)
            .join("Phase.db2");

        let reader = Wdc4Reader::open(&path)
            .with_context(|| format!("failed to open {}", path.display()))?;

        let mut entries = HashMap::with_capacity(reader.total_count());
        for (id, idx) in reader.iter_records() {
            entries.insert(
                id,
                PhaseEntry {
                    id,
                    // WDC4 record ids supply C++ field 0 (`ID`).
                    flags: reader.get_field_u16(idx, 0),
                },
            );
        }

        info!("Loaded {} phases from {}", entries.len(), path.display());
        Ok(Self { entries })
    }

    pub fn apply_hotfix_entries_like_cpp(
        &mut self,
        entries: impl IntoIterator<Item = PhaseEntry>,
    ) -> usize {
        let mut count = 0usize;
        for entry in entries {
            self.entries.insert(entry.id, entry);
            count += 1;
        }
        count
    }

    pub fn get(&self, id: u32) -> Option<&PhaseEntry> {
        self.entries.get(&id)
    }

    pub fn entries(&self) -> impl Iterator<Item = &PhaseEntry> {
        self.entries.values()
    }

    pub fn contains(&self, id: u32) -> bool {
        self.entries.contains_key(&id)
    }

    /// C++ `PhasingHandler::IsPersonalPhase`.
    pub fn is_personal_phase(&self, phase_id: u32) -> bool {
        self.get(phase_id)
            .map(|phase| phase.flags & PHASE_ENTRY_FLAG_PERSONAL != 0)
            .unwrap_or(false)
    }

    /// C++ `PhasingHandler.cpp::GetPhaseFlags` cosmetic branch.
    pub fn is_cosmetic_phase(&self, phase_id: u32) -> bool {
        self.get(phase_id)
            .map(|phase| phase.flags & PHASE_ENTRY_FLAG_COSMETIC != 0)
            .unwrap_or(false)
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

pub struct PhaseGroupStore {
    entries: HashMap<u32, PhaseXPhaseGroupEntry>,
    phases_by_group: HashMap<u32, Vec<u32>>,
}

impl PhaseGroupStore {
    pub fn from_entries(
        phase_store: &PhaseStore,
        entries: impl IntoIterator<Item = PhaseXPhaseGroupEntry>,
    ) -> Self {
        let entries: HashMap<_, _> = entries.into_iter().map(|entry| (entry.id, entry)).collect();
        let phases_by_group = build_phases_by_group_like_cpp(phase_store, entries.values());
        Self {
            entries,
            phases_by_group,
        }
    }

    /// Load PhaseXPhaseGroup.db2 from `{data_dir}/dbc/{locale}/PhaseXPhaseGroup.db2`.
    ///
    /// C++ `DB2Manager::LoadStores` only inserts rows whose PhaseID exists in
    /// `sPhaseStore`.
    pub fn load(data_dir: &str, locale: &str, phase_store: &PhaseStore) -> Result<Self> {
        let path = Path::new(data_dir)
            .join("dbc")
            .join(locale)
            .join("PhaseXPhaseGroup.db2");

        let reader = Wdc4Reader::open(&path)
            .with_context(|| format!("failed to open {}", path.display()))?;

        let mut entries = HashMap::with_capacity(reader.total_count());
        for (id, idx) in reader.iter_records() {
            entries.insert(
                id,
                PhaseXPhaseGroupEntry {
                    id,
                    // WDC4 record ids supply C++ field 0 (`ID`). C++ field 2
                    // (`PhaseGroupID`) is DB2Meta parent index 1.
                    phase_id: reader.get_field_u16(idx, 0),
                    phase_group_id: reader.get_relationship_id(idx).unwrap_or(0),
                },
            );
        }

        let phases_by_group = build_phases_by_group_like_cpp(phase_store, entries.values());
        info!(
            "Loaded {} phase-group rows and {} phase groups from {}",
            entries.len(),
            phases_by_group.len(),
            path.display()
        );
        Ok(Self {
            entries,
            phases_by_group,
        })
    }

    pub fn apply_hotfix_entries_like_cpp(
        &mut self,
        phase_store: &PhaseStore,
        entries: impl IntoIterator<Item = PhaseXPhaseGroupEntry>,
    ) -> usize {
        let mut count = 0usize;
        for entry in entries {
            self.entries.insert(entry.id, entry);
            count += 1;
        }

        self.phases_by_group = build_phases_by_group_like_cpp(phase_store, self.entries.values());
        count
    }

    pub fn get(&self, id: u32) -> Option<&PhaseXPhaseGroupEntry> {
        self.entries.get(&id)
    }

    /// C++ `DB2Manager::GetPhasesForGroup`.
    pub fn phases_for_group(&self, group_id: u32) -> Option<&[u32]> {
        self.phases_by_group.get(&group_id).map(Vec::as_slice)
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

fn build_phases_by_group_like_cpp<'a>(
    phase_store: &PhaseStore,
    entries: impl IntoIterator<Item = &'a PhaseXPhaseGroupEntry>,
) -> HashMap<u32, Vec<u32>> {
    let mut phases_by_group = HashMap::<u32, Vec<u32>>::new();
    for entry in entries {
        if phase_store.contains(u32::from(entry.phase_id)) {
            phases_by_group
                .entry(entry.phase_group_id)
                .or_default()
                .push(u32::from(entry.phase_id));
        }
    }
    phases_by_group
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn phase_store_reports_personal_and_cosmetic_flags_like_cpp() {
        let store = PhaseStore::from_entries([
            PhaseEntry {
                id: 10,
                flags: PHASE_ENTRY_FLAG_PERSONAL,
            },
            PhaseEntry {
                id: 20,
                flags: PHASE_ENTRY_FLAG_COSMETIC,
            },
        ]);

        assert!(store.is_personal_phase(10));
        assert!(!store.is_personal_phase(20));
        assert!(store.is_cosmetic_phase(20));
        assert!(!store.is_cosmetic_phase(999));
    }

    #[test]
    fn phase_group_store_skips_missing_phase_ids_like_cpp_manager() {
        let phase_store = PhaseStore::from_entries([PhaseEntry { id: 10, flags: 0 }]);
        let group_store = PhaseGroupStore::from_entries(
            &phase_store,
            [
                PhaseXPhaseGroupEntry {
                    id: 1,
                    phase_id: 10,
                    phase_group_id: 5,
                },
                PhaseXPhaseGroupEntry {
                    id: 2,
                    phase_id: 11,
                    phase_group_id: 5,
                },
            ],
        );

        assert_eq!(group_store.phases_for_group(5), Some([10].as_slice()));
        assert!(group_store.phases_for_group(6).is_none());
    }

    #[test]
    fn phase_hotfix_entries_replace_the_effective_db2_row_like_cpp() {
        let mut store = PhaseStore::from_entries([PhaseEntry { id: 10, flags: 0 }]);

        assert_eq!(
            store.apply_hotfix_entries_like_cpp([
                PhaseEntry {
                    id: 10,
                    flags: PHASE_ENTRY_FLAG_PERSONAL,
                },
                PhaseEntry {
                    id: 20,
                    flags: PHASE_ENTRY_FLAG_COSMETIC,
                },
            ]),
            2
        );
        assert!(store.is_personal_phase(10));
        assert!(store.is_cosmetic_phase(20));
    }

    #[test]
    fn phase_group_hotfix_entries_rebuild_the_validated_group_index_like_cpp() {
        let phases = PhaseStore::from_entries([
            PhaseEntry { id: 10, flags: 0 },
            PhaseEntry { id: 20, flags: 0 },
        ]);
        let mut groups = PhaseGroupStore::from_entries(
            &phases,
            [PhaseXPhaseGroupEntry {
                id: 1,
                phase_id: 10,
                phase_group_id: 5,
            }],
        );

        assert_eq!(
            groups.apply_hotfix_entries_like_cpp(
                &phases,
                [
                    PhaseXPhaseGroupEntry {
                        id: 1,
                        phase_id: 20,
                        phase_group_id: 6,
                    },
                    PhaseXPhaseGroupEntry {
                        id: 2,
                        phase_id: 999,
                        phase_group_id: 6,
                    },
                ],
            ),
            2
        );
        assert!(groups.phases_for_group(5).is_none());
        assert_eq!(groups.phases_for_group(6), Some([20].as_slice()));
    }

    #[test]
    fn load_phase_db2_when_fixture_exists() {
        let data_dir = "/home/server/woltk-server-core/Data";
        let locale = "esES";
        let path = Path::new(data_dir)
            .join("dbc")
            .join(locale)
            .join("Phase.db2");
        if !path.exists() {
            eprintln!("Skipping test: Phase.db2 not found at {}", path.display());
            return;
        }

        let store = PhaseStore::load(data_dir, locale).expect("failed to load Phase.db2");
        assert!(!store.is_empty());
    }

    #[test]
    fn load_phase_x_phase_group_db2_when_fixture_exists() {
        let data_dir = "/home/server/woltk-server-core/Data";
        let locale = "esES";
        let phase_path = Path::new(data_dir)
            .join("dbc")
            .join(locale)
            .join("Phase.db2");
        let group_path = Path::new(data_dir)
            .join("dbc")
            .join(locale)
            .join("PhaseXPhaseGroup.db2");
        if !phase_path.exists() || !group_path.exists() {
            eprintln!(
                "Skipping test: phase DB2 fixtures not found at {} / {}",
                phase_path.display(),
                group_path.display()
            );
            return;
        }

        let phase_store = PhaseStore::load(data_dir, locale).expect("failed to load Phase.db2");
        let group_store = PhaseGroupStore::load(data_dir, locale, &phase_store)
            .expect("failed to load PhaseXPhaseGroup.db2");
        assert!(!group_store.is_empty());
    }
}
