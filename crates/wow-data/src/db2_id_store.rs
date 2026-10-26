// Copyright (c) 2026 alseif0x
// RustyCore - WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 - https://www.gnu.org/licenses/gpl-3.0.html

//! Minimal DB2 ID stores for C++ `LookupEntry(id)` validation.

use std::collections::HashSet;
use std::path::Path;

use anyhow::{Context, Result, ensure};
use tracing::info;

use crate::wdc4::Wdc4Reader;

#[derive(Debug, Clone)]
pub struct Db2IdStore {
    name: &'static str,
    ids: HashSet<u32>,
    // Only populated by the complete loader. Legacy file-only stores retain
    // their existing semantics and cannot be used for effective hotfix removal.
    table_hash: Option<u32>,
}

impl Db2IdStore {
    pub fn from_ids(name: &'static str, ids: impl IntoIterator<Item = u32>) -> Self {
        Self {
            name,
            ids: ids.into_iter().collect(),
            table_hash: None,
        }
    }

    pub fn load(data_dir: &str, locale: &str, filename: &'static str) -> Result<Self> {
        Self::load_inner(data_dir, locale, filename, false)
    }

    /// Load every WDC4 record and retain its actual header table hash.
    ///
    /// Unlike `load`, this rejects a reader that skipped unresolved copy
    /// sources, because its absent IDs would otherwise be unproven.
    pub(super) fn load_complete(
        data_dir: &str,
        locale: &str,
        filename: &'static str,
    ) -> Result<Self> {
        Self::load_inner(data_dir, locale, filename, true)
    }

    fn load_inner(
        data_dir: &str,
        locale: &str,
        filename: &'static str,
        require_complete: bool,
    ) -> Result<Self> {
        let path = Path::new(data_dir).join("dbc").join(locale).join(filename);
        let reader = Wdc4Reader::open(&path)
            .with_context(|| format!("failed to open {}", path.display()))?;
        let mut ids = HashSet::with_capacity(reader.total_count());
        let mut observed_records = 0;
        for (id, _) in reader.iter_records() {
            observed_records += 1;
            ids.insert(id);
        }
        if require_complete {
            ensure!(
                observed_records == reader.total_count(),
                "{} contains unresolved WDC4 copy records: observed {observed_records} of {}",
                path.display(),
                reader.total_count()
            );
        }

        info!("Loaded {} rows from {}", ids.len(), path.display());
        Ok(Self {
            name: filename,
            ids,
            table_hash: require_complete.then(|| reader.table_hash()),
        })
    }

    pub(super) fn apply_effective_overlay(
        &mut self,
        added_ids: impl IntoIterator<Item = u32>,
        removals: &crate::Db2HotfixRemovalStoreLikeCpp,
    ) -> Result<()> {
        let table_hash = self
            .table_hash
            .context("complete DB2 ID store is missing its WDC4 table hash")?;
        self.ids.extend(added_ids);
        self.ids.retain(|id| {
            // The hotfix metadata stores RecordID as i32; preserve all ID bits.
            !removals.contains_like_cpp(table_hash, i32::from_ne_bytes(id.to_ne_bytes()))
        });
        Ok(())
    }

    pub fn contains(&self, id: u32) -> bool {
        self.ids.contains(&id)
    }

    pub fn len(&self) -> usize {
        self.ids.len()
    }

    pub fn is_empty(&self) -> bool {
        self.ids.is_empty()
    }

    pub const fn name(&self) -> &'static str {
        self.name
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn db2_id_store_indexes_record_ids_like_cpp_store() {
        let store = Db2IdStore::from_ids("Example.db2", [1, 7, 42]);

        assert_eq!(store.name(), "Example.db2");
        assert!(store.contains(7));
        assert!(!store.contains(8));
        assert_eq!(store.len(), 3);
    }
}

#[cfg(test)]
#[path = "db2_id_store/tests.rs"]
mod completeness_tests;
