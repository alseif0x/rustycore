//! Immutable target hotfix delivery catalog.
//! 02245dcd DB2Stores::LoadDB2, DB2DatabaseLoader::Load, LoadHotfixData,
//! DB2StorageBase::WriteRecord. TactKey has an external ID and 16 unsigned
//! byte fields on the wire; that ID is NOT prepended to the serialized data.

use crate::{HotfixBlobCache, hotfix_cache::HotfixRecordStatus, wdc4::Wdc4Reader};
use anyhow::{Result, ensure};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};

pub const TACT_KEY_TABLE_HASH: u32 = 0xDF2F53CF;
pub const TACT_KEY_LAYOUT_HASH: u32 = 0xCBA490FC;

#[cfg(test)]
#[path = "forever_hotfix/load_tests.rs"]
mod load_tests;

/// No Debug/Clone: canonical effective values are private client/server data.
pub struct ForeverTactKeys {
    records: BTreeMap<u32, [u8; 16]>,
}

impl ForeverTactKeys {
    /// A real normal-CASC baseline is mandatory. SQL-only stores are not
    /// evidence that an unlisted baseline record is absent.
    pub fn load(directory: &Path) -> Result<Self> {
        let reader = Wdc4Reader::open(&directory.join("TactKey.db2"))?;
        ensure!(
            reader.format_version() == 5
                && reader.table_hash() == TACT_KEY_TABLE_HASH
                && reader.layout_hash() == TACT_KEY_LAYOUT_HASH
                && reader.field_count() == 1
                && reader.declared_field_count() == 1
                && reader.parent_lookup_count() == 0,
            "Wrong target TactKey schema"
        );
        ensure!(
            reader.inline_id_field().is_none(),
            "TactKey needs external IDs"
        );
        let mut records = BTreeMap::new();
        for (id, index) in reader.iter_records() {
            ensure!(
                reader
                    .record_bytes(index)
                    .is_some_and(|record| record.len() == 16),
                "TactKey needs captured uncompressed 16-byte records"
            );
            let key = reader.get_fixed_u8_array::<16>(index, 0)?;
            ensure!(
                records.insert(id, key).is_none(),
                "Duplicate TactKey baseline ID"
            );
        }
        ensure!(
            records.len() == reader.total_count(),
            "Unresolved TactKey copy record"
        );
        Ok(Self { records })
    }

    /// Official SQL replaces/adds baseline entries; custom SQL is applied last.
    /// Consume each batch so no mutable mirror survives catalog publication.
    pub fn with_overlays(
        mut self,
        official: impl IntoIterator<Item = (u32, [u8; 16])>,
        custom: impl IntoIterator<Item = (u32, [u8; 16])>,
    ) -> Result<Self> {
        for batch in [
            official.into_iter().collect::<Vec<_>>(),
            custom.into_iter().collect(),
        ] {
            let mut seen = BTreeSet::new();
            for (id, key) in batch {
                ensure!(seen.insert(id), "Duplicate TactKey SQL batch ID");
                self.records.insert(id, key);
            }
        }
        Ok(self)
    }

    pub fn count(&self) -> usize {
        self.records.len()
    }
}

/// A known but unported store must not masquerade as missing data.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RecordError {
    UnportedStore,
}

pub struct ForeverHotfixCatalog {
    metadata: HotfixBlobCache,
    tact_keys: ForeverTactKeys,
}

impl ForeverHotfixCatalog {
    pub fn new(mut tact_keys: ForeverTactKeys, metadata: HotfixBlobCache) -> Result<Self> {
        ensure!(
            metadata.has_table(TACT_KEY_TABLE_HASH),
            "TactKey store must be registered before hotfix metadata loading"
        );
        // Source ORDER BY Id: the last advertised status for each record wins.
        // Only RecordRemoved erases a typed record; a later Valid cancels an
        // earlier removal. Do not delete on Invalid/NotPublic/NotSet.
        let mut deleted = BTreeMap::new();
        for push in metadata.hotfix_pushes().values() {
            for record in &push.records {
                if record.table_hash == TACT_KEY_TABLE_HASH {
                    deleted.insert(
                        record.record_id as u32,
                        record.status == HotfixRecordStatus::RecordRemoved,
                    );
                }
            }
        }
        for (id, remove) in deleted {
            if remove {
                tact_keys.records.remove(&id);
            }
        }
        Ok(Self {
            metadata,
            tact_keys,
        })
    }

    pub fn metadata(&self) -> &HotfixBlobCache {
        &self.metadata
    }
    pub fn tact_key_count(&self) -> usize {
        self.tact_keys.count()
    }
    pub fn has_store(&self, hash: u32) -> bool {
        self.metadata.has_table(hash)
    }

    /// Typed WriteRecord equivalent. Unknown stores/missing typed records are
    /// genuinely absent; unported known stores are explicit integration errors.
    /// TactKey permits no optional data in this source (the allowed TactKey key
    /// belongs to BroadcastText, not to TactKey itself).
    pub fn record(&self, hash: u32, id: u32) -> Result<Option<&[u8]>, RecordError> {
        if hash == TACT_KEY_TABLE_HASH {
            return Ok(self.tact_keys.records.get(&id).map(|key| key.as_slice()));
        }
        if self.has_store(hash) {
            return Err(RecordError::UnportedStore);
        }
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn baseline() -> ForeverTactKeys {
        // Synthetic values only; actual client keys are never fixtures.
        ForeverTactKeys {
            records: BTreeMap::from([(7, [1; 16]), (8, [2; 16])]),
        }
    }
    fn metadata() -> HotfixBlobCache {
        let mut metadata = HotfixBlobCache::new();
        metadata.register_typed_table(TACT_KEY_TABLE_HASH);
        metadata
    }

    #[test]
    fn official_then_custom_overlay_preserves_baseline_and_adds_new_records() {
        let keys = baseline()
            .with_overlays([(7, [3; 16]), (9, [4; 16])], [(7, [5; 16])])
            .unwrap();
        let catalog = ForeverHotfixCatalog::new(keys, metadata()).unwrap();
        assert_eq!(catalog.tact_key_count(), 3);
        assert_eq!(
            catalog.record(TACT_KEY_TABLE_HASH, 7).unwrap(),
            Some([5; 16].as_slice())
        );
        assert_eq!(
            catalog.record(TACT_KEY_TABLE_HASH, 8).unwrap(),
            Some([2; 16].as_slice())
        );
        assert!(catalog.record(TACT_KEY_TABLE_HASH, 10).unwrap().is_none());
        assert_eq!(catalog.metadata().total_blobs(), 0); // no second raw mirror
    }

    #[test]
    fn duplicate_unordered_sql_rows_and_unregistered_store_fail_closed() {
        assert!(
            baseline()
                .with_overlays([(7, [1; 16]), (7, [2; 16])], [])
                .is_err()
        );
        assert!(
            baseline()
                .with_overlays([], [(7, [1; 16]), (7, [2; 16])])
                .is_err()
        );
        assert!(ForeverHotfixCatalog::new(baseline(), HotfixBlobCache::new()).is_err());
    }

    #[test]
    fn only_the_last_record_removed_status_erases_a_typed_record() {
        let mut metadata = metadata();
        metadata.apply_hotfix_data_rows_like_cpp(
            [
                (1, 1, TACT_KEY_TABLE_HASH, 7, 2),
                (2, 2, TACT_KEY_TABLE_HASH, 7, 1),
                (3, 3, TACT_KEY_TABLE_HASH, 8, 2),
            ],
            "esES",
        );
        let catalog = ForeverHotfixCatalog::new(baseline(), metadata).unwrap();
        assert!(catalog.record(TACT_KEY_TABLE_HASH, 7).unwrap().is_some());
        assert!(catalog.record(TACT_KEY_TABLE_HASH, 8).unwrap().is_none());
        for status in [0, 1, 3, 4] {
            let mut metadata = super::tests::metadata();
            metadata
                .apply_hotfix_data_rows_like_cpp([(1, 1, TACT_KEY_TABLE_HASH, 7, status)], "esES");
            assert!(
                ForeverHotfixCatalog::new(baseline(), metadata)
                    .unwrap()
                    .record(TACT_KEY_TABLE_HASH, 7)
                    .unwrap()
                    .is_some()
            );
        }
    }

    #[test]
    fn unknown_is_absent_but_known_unported_is_not_absent() {
        let mut metadata = metadata();
        metadata.register_typed_table(123);
        let catalog = ForeverHotfixCatalog::new(baseline(), metadata).unwrap();
        assert!(catalog.record(99, 1).unwrap().is_none());
        assert_eq!(catalog.record(123, 1), Err(RecordError::UnportedStore));
    }
}
