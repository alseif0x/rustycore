// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! In-memory cache of raw DB2 record blobs for serving DBReply (SMSG_DB_REPLY).
//!
//! When the client sends `CMSG_DB_QUERY_BULK` for records it does not have in
//! its local DB2 cache, the server must respond with the raw binary blob for
//! each requested record.  This module pre-loads record blobs from `.db2`
//! files at startup so they can be looked up with O(1) cost at runtime.

use std::collections::BTreeMap;
use std::collections::HashMap;
use std::collections::HashSet;
use std::fs;
use std::path::Path;

use anyhow::Result;
use tracing::info;
use wow_database::{HotfixDatabase, HotfixStatements};

use crate::wdc4::Wdc4Reader;

/// C++ `DB2Manager::HotfixId`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct HotfixId {
    pub push_id: i32,
    pub unique_id: u32,
}

/// C++ `DB2Manager::HotfixRecord::Status`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum HotfixRecordStatus {
    NotSet = 0,
    Valid = 1,
    RecordRemoved = 2,
    Invalid = 3,
    NotPublic = 4,
}

impl From<u8> for HotfixRecordStatus {
    fn from(value: u8) -> Self {
        match value {
            1 => Self::Valid,
            2 => Self::RecordRemoved,
            3 => Self::Invalid,
            4 => Self::NotPublic,
            _ => Self::NotSet,
        }
    }
}

/// C++ `DB2Manager::HotfixRecord`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HotfixRecord {
    pub table_hash: u32,
    pub record_id: i32,
    pub id: HotfixId,
    pub status: HotfixRecordStatus,
    pub available_locales_mask: u32,
}

/// C++ `DB2Manager::HotfixPush`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HotfixPush {
    pub records: Vec<HotfixRecord>,
    pub available_locales_mask: u32,
}

/// C++ `DB2Manager::HotfixOptionalData`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HotfixOptionalData {
    pub key: u32,
    pub data: Vec<u8>,
}

/// Cached raw record bytes indexed by `(table_hash, record_id)`.
#[derive(Default)]
pub struct HotfixBlobCache {
    /// Outer key: table_hash (from DB2 header).
    /// Inner key: record_id.
    /// Value: raw record bytes (inline strings, no copy-table dedup).
    blobs: HashMap<u32, HashMap<u32, Vec<u8>>>,
    /// `(table_hash, record_id)` pairs whose blob came from the `hotfix_blob`
    /// DB table (TC-population pre-decoded wire bytes) rather than a raw `.db2`
    /// file (WDC4 bitpacked). Only these are safe to serve in
    /// SMSG_HOTFIX_CONNECT — raw `.db2` bytes have the wrong size/shape and
    /// desync the client's content cursor (ERROR#132).
    hotfix_blob_keys: HashSet<(u32, i32)>,
    hotfix_data: BTreeMap<i32, HotfixPush>,
    optional_data: HashMap<String, HashMap<(u32, i32), Vec<HotfixOptionalData>>>,
    max_hotfix_id: i32,
}

impl HotfixBlobCache {
    /// Create an empty cache.
    pub fn new() -> Self {
        Self::default()
    }

    /// Load every record from a `.db2` file and store the raw bytes.
    ///
    /// The `table_hash` is read from the DB2 header, so you don't have to
    /// supply it explicitly.
    pub fn load_db2<P: AsRef<Path>>(&mut self, path: P) -> Result<usize> {
        let path = path.as_ref();
        let reader = Wdc4Reader::open(path)
            .map_err(|e| anyhow::anyhow!("failed to open {}: {}", path.display(), e))?;

        let table_hash = reader.table_hash();
        let table = self.blobs.entry(table_hash).or_default();

        let mut count = 0usize;
        for (record_id, record_idx) in reader.iter_records() {
            if let Some(bytes) = reader.record_bytes(record_idx) {
                table.insert(record_id, bytes.to_vec());
                count += 1;
            }
        }

        Ok(count)
    }

    /// Insert or replace one raw DB2 blob.
    pub fn insert_blob(&mut self, table_hash: u32, record_id: i32, bytes: Vec<u8>) {
        self.blobs
            .entry(table_hash)
            .or_default()
            .insert(record_id as u32, bytes);
        // Mark this record as `hotfix_blob`-sourced (pre-decoded). `.db2` files
        // are loaded via `load_db2` which inserts directly into `blobs` and
        // bypasses this method, so they are NOT marked servable.
        self.hotfix_blob_keys.insert((table_hash, record_id));
    }

    /// Load C++ `hotfix_blob` rows from the Hotfix database for one locale.
    pub async fn load_hotfix_blobs_from_db(
        &mut self,
        db: &HotfixDatabase,
        locale: &str,
    ) -> Result<usize> {
        let stmt = db.prepare(HotfixStatements::SEL_HOTFIX_BLOB);
        let mut result = db.query(&stmt).await?;
        if result.is_empty() {
            return Ok(0);
        }

        let mut count = 0usize;
        loop {
            let table_hash: u32 = result.read(0);
            let record_id: i32 = result.read(1);
            let row_locale = result.read_string(2);
            let blob: Vec<u8> = result.try_read(3).unwrap_or_default();

            if row_locale == locale {
                self.insert_blob(table_hash, record_id, blob);
                count += 1;
            }

            if !result.next_row() {
                break;
            }
        }

        Ok(count)
    }

    /// Load C++ `hotfix_data` rows and group them by push id.
    pub async fn load_hotfix_data_from_db(
        &mut self,
        db: &HotfixDatabase,
        locale: &str,
    ) -> Result<usize> {
        let stmt = db.prepare(HotfixStatements::SEL_HOTFIX_DATA);
        let mut result = db.query(&stmt).await?;
        if result.is_empty() {
            return Ok(0);
        }

        let locale_mask = locale_mask_for_name(locale);
        let mut count = 0usize;
        loop {
            let push_id: i32 = result.read(0);
            let unique_id: u32 = result.read(1);
            let table_hash: u32 = result.read(2);
            let record_id: i32 = result.read(3);
            let status = HotfixRecordStatus::from(result.read::<u8>(4));

            // rustycore can only serialize PRE-DECODED `hotfix_blob` records for
            // SMSG_HOTFIX_CONNECT. Raw `.db2` (WDC4 bitpacked) bytes have the
            // wrong size/shape and desync the client's content cursor →
            // WowDB2Lookup negative-count crash (ERROR#132). Keep a record only
            // when it is Valid AND backed by a `hotfix_blob` row; drop the rest
            // (raw `.db2`-only Valid, RecordRemoved, Invalid, NotPublic) so the
            // client uses its own correct local DB2 copy — matching the real
            // 54261 server, which serves only Valid records it can fully encode.
            if !self.is_servable_hotfix_record(status, table_hash, record_id) {
                if !result.next_row() {
                    break;
                }
                continue;
            }

            let record = HotfixRecord {
                table_hash,
                record_id,
                id: HotfixId { push_id, unique_id },
                status,
                available_locales_mask: locale_mask,
            };

            let push = self.hotfix_data.entry(push_id).or_default();
            push.available_locales_mask |= record.available_locales_mask;
            push.records.push(record);
            self.max_hotfix_id = self.max_hotfix_id.max(push_id);
            count += 1;

            if !result.next_row() {
                break;
            }
        }

        Ok(count)
    }

    /// Load C++ `hotfix_optional_data` rows for one locale.
    pub async fn load_hotfix_optional_data_from_db(
        &mut self,
        db: &HotfixDatabase,
        locale: &str,
    ) -> Result<usize> {
        let stmt = db.prepare(HotfixStatements::SEL_HOTFIX_OPTIONAL_DATA);
        let mut result = db.query(&stmt).await?;
        if result.is_empty() {
            return Ok(0);
        }

        let mut count = 0usize;
        loop {
            let table_hash: u32 = result.read(0);
            let record_id: i32 = result.read(1);
            let row_locale = result.read_string(2);
            let key: u32 = result.read(3);
            let data: Vec<u8> = result.try_read(4).unwrap_or_default();

            if row_locale == locale {
                self.optional_data
                    .entry(row_locale)
                    .or_default()
                    .entry((table_hash, record_id))
                    .or_default()
                    .push(HotfixOptionalData { key, data });
                count += 1;
            }

            if !result.next_row() {
                break;
            }
        }

        Ok(count)
    }

    /// Look up the raw blob for a `(table_hash, record_id)` pair.
    pub fn get(&self, table_hash: u32, record_id: i32) -> Option<&[u8]> {
        let table = self.blobs.get(&table_hash)?;
        table.get(&(record_id as u32)).map(|v| v.as_slice())
    }

    /// Look up C++ `DB2Manager::HotfixOptionalData` entries for one locale.
    pub fn get_optional_data(
        &self,
        table_hash: u32,
        record_id: i32,
        locale: &str,
    ) -> Option<&[HotfixOptionalData]> {
        self.optional_data
            .get(locale)?
            .get(&(table_hash, record_id))
            .map(|entries| entries.as_slice())
    }

    /// Whether the cache has any data for a given table hash.
    pub fn has_table(&self, table_hash: u32) -> bool {
        self.blobs.contains_key(&table_hash)
    }

    /// Whether `(table_hash, record_id)` is backed by a `hotfix_blob` row
    /// (pre-decoded wire bytes), as opposed to a raw `.db2` record. Only these
    /// can be served correctly in SMSG_HOTFIX_CONNECT.
    pub fn has_hotfix_blob(&self, table_hash: u32, record_id: i32) -> bool {
        self.hotfix_blob_keys.contains(&(table_hash, record_id))
    }

    /// Whether a `hotfix_data` record may be advertised/served in
    /// SMSG_HOTFIX_CONNECT: only `Valid` records backed by a pre-decoded
    /// `hotfix_blob` row. Raw `.db2` (WDC4 bitpacked) records, `RecordRemoved`,
    /// `Invalid`, and `NotPublic` are all dropped so the client falls back to
    /// its own correct local DB2 copy and never desyncs (ERROR#132).
    pub fn is_servable_hotfix_record(
        &self,
        status: HotfixRecordStatus,
        table_hash: u32,
        record_id: i32,
    ) -> bool {
        status == HotfixRecordStatus::Valid && self.has_hotfix_blob(table_hash, record_id)
    }

    /// Total number of blobs cached across all tables.
    pub fn total_blobs(&self) -> usize {
        self.blobs.values().map(|t| t.len()).sum()
    }

    pub fn hotfix_pushes(&self) -> &BTreeMap<i32, HotfixPush> {
        &self.hotfix_data
    }

    pub fn hotfix_push(&self, push_id: i32) -> Option<&HotfixPush> {
        self.hotfix_data.get(&push_id)
    }

    pub fn available_hotfix_ids(&self, locale: &str) -> Vec<HotfixId> {
        let locale_mask = locale_mask_for_name(locale);
        self.hotfix_data
            .values()
            .filter(|push| push.available_locales_mask & locale_mask != 0)
            .filter_map(|push| push.records.first().map(|record| record.id))
            .collect()
    }

    pub fn hotfix_count(&self) -> usize {
        self.hotfix_data
            .values()
            .map(|push| push.records.len())
            .sum()
    }

    pub fn max_hotfix_id(&self) -> i32 {
        self.max_hotfix_id
    }
}

fn locale_mask_for_name(locale: &str) -> u32 {
    locale_index(locale).map_or(0, |index| 1_u32 << index)
}

pub fn hotfix_locale_mask(locale: &str) -> u32 {
    locale_mask_for_name(locale)
}

fn locale_index(locale: &str) -> Option<u32> {
    match locale {
        "enUS" => Some(0),
        "koKR" => Some(1),
        "frFR" => Some(2),
        "deDE" => Some(3),
        "zhCN" => Some(4),
        "zhTW" => Some(5),
        "esES" => Some(6),
        "esMX" => Some(7),
        "ruRU" => Some(8),
        "jaJP" => Some(9),
        "ptBR" => Some(10),
        "itIT" => Some(11),
        _ => None,
    }
}

/// Helper: load `Item.db2` + `ItemSparse.db2` (and any other needed files) and log progress.
pub fn build_hotfix_blob_cache(data_dir: &str, locale: &str) -> HotfixBlobCache {
    let mut cache = HotfixBlobCache::new();

    let dbc_dir = Path::new(data_dir).join("dbc").join(locale);

    let mut loaded_files = 0usize;
    let mut failed_files = 0usize;
    let mut loaded_records = 0usize;

    match fs::read_dir(&dbc_dir) {
        Ok(entries) => {
            let mut db2_paths = entries
                .filter_map(|entry| entry.ok().map(|entry| entry.path()))
                .filter(|path| {
                    path.extension()
                        .and_then(|ext| ext.to_str())
                        .is_some_and(|ext| ext.eq_ignore_ascii_case("db2"))
                })
                .collect::<Vec<_>>();
            db2_paths.sort();

            for db2_path in db2_paths {
                match cache.load_db2(&db2_path) {
                    Ok(n) => {
                        loaded_files += 1;
                        loaded_records += n;
                    }
                    Err(e) => {
                        failed_files += 1;
                        tracing::warn!(
                            "HotfixBlobCache: failed to load {}: {e}",
                            db2_path.display()
                        );
                    }
                }
            }
        }
        Err(e) => tracing::warn!(
            "HotfixBlobCache: failed to read DB2 directory {}: {e}",
            dbc_dir.display()
        ),
    }

    info!(
        "HotfixBlobCache: loaded {loaded_records} records from {loaded_files} DB2 files ({failed_files} failed)"
    );
    info!(
        "HotfixBlobCache: {} total blobs cached across {} table hashes",
        cache.total_blobs(),
        cache.blobs.len()
    );
    cache
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hotfix_record_status_matches_cpp_values() {
        assert_eq!(HotfixRecordStatus::from(0), HotfixRecordStatus::NotSet);
        assert_eq!(HotfixRecordStatus::from(1), HotfixRecordStatus::Valid);
        assert_eq!(
            HotfixRecordStatus::from(2),
            HotfixRecordStatus::RecordRemoved
        );
        assert_eq!(HotfixRecordStatus::from(3), HotfixRecordStatus::Invalid);
        assert_eq!(HotfixRecordStatus::from(4), HotfixRecordStatus::NotPublic);
    }

    #[test]
    fn locale_masks_match_dbc_locale_order() {
        assert_eq!(locale_mask_for_name("enUS"), 1);
        assert_eq!(locale_mask_for_name("esES"), 1 << 6);
        assert_eq!(locale_mask_for_name("itIT"), 1 << 11);
        assert_eq!(locale_mask_for_name("bad"), 0);
    }

    #[test]
    fn insert_blob_indexes_by_table_hash_and_record_id() {
        let mut cache = HotfixBlobCache::new();
        cache.insert_blob(0x919B_E54E, 58256, vec![1, 2, 3]);

        assert_eq!(cache.get(0x919B_E54E, 58256), Some(&[1, 2, 3][..]));
        assert!(cache.has_table(0x919B_E54E));
        assert_eq!(cache.total_blobs(), 1);
    }

    #[test]
    fn insert_blob_marks_record_as_hotfix_blob_sourced() {
        let mut cache = HotfixBlobCache::new();
        cache.insert_blob(0x919B_E54E, 58256, vec![1, 2, 3]);

        // A record inserted via insert_blob (the hotfix_blob path) is marked.
        assert!(cache.has_hotfix_blob(0x919B_E54E, 58256));
        // A record never inserted via insert_blob (e.g. a raw `.db2` record
        // loaded by load_db2, which inserts directly into `blobs`) is NOT
        // marked — even in the same, already-present table.
        assert!(cache.has_table(0x919B_E54E));
        assert!(!cache.has_hotfix_blob(0x919B_E54E, 99999));
        assert!(!cache.has_hotfix_blob(0x1234_5678, 58256));
    }

    #[test]
    fn only_valid_hotfix_blob_records_are_servable() {
        let mut cache = HotfixBlobCache::new();
        cache.insert_blob(0x919B_E54E, 58256, vec![1, 2, 3]);

        // Positive: Valid + backed by a hotfix_blob row → servable (kept).
        assert!(cache.is_servable_hotfix_record(HotfixRecordStatus::Valid, 0x919B_E54E, 58256));
        // Negative: Valid but no hotfix_blob (raw `.db2`-only) → dropped; raw
        // WDC4 bitpacked bytes would desync the client's content cursor
        // (ERROR#132).
        assert!(!cache.is_servable_hotfix_record(HotfixRecordStatus::Valid, 0x919B_E54E, 99999));
        // Negative: RecordRemoved is never served (it would tell the client to
        // delete a DB2 record it needs), even when a blob is present.
        assert!(!cache.is_servable_hotfix_record(
            HotfixRecordStatus::RecordRemoved,
            0x919B_E54E,
            58256
        ));
        // Negative: Invalid is dropped too — declare == serve.
        assert!(!cache.is_servable_hotfix_record(HotfixRecordStatus::Invalid, 0x919B_E54E, 58256));
    }

    #[test]
    fn optional_data_is_indexed_by_locale_table_and_record() {
        let mut cache = HotfixBlobCache::new();
        cache
            .optional_data
            .entry("enUS".to_string())
            .or_default()
            .entry((0xDF2F_53CF, 67))
            .or_default()
            .push(HotfixOptionalData {
                key: 0x1234_5678,
                data: vec![9, 8, 7],
            });

        let entries = cache
            .get_optional_data(0xDF2F_53CF, 67, "enUS")
            .expect("optional data should exist");
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].key, 0x1234_5678);
        assert_eq!(entries[0].data, [9, 8, 7]);
        assert!(cache.get_optional_data(0xDF2F_53CF, 67, "esES").is_none());
    }
}
