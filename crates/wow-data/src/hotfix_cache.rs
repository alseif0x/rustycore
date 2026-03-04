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

use std::collections::HashMap;
use std::path::Path;

use anyhow::Result;
use tracing::info;

use crate::wdc4::Wdc4Reader;

/// Cached raw record bytes indexed by `(table_hash, record_id)`.
#[derive(Default)]
pub struct HotfixBlobCache {
    /// Outer key: table_hash (from DB2 header).
    /// Inner key: record_id.
    /// Value: raw record bytes (inline strings, no copy-table dedup).
    blobs: HashMap<u32, HashMap<u32, Vec<u8>>>,
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

    /// Look up the raw blob for a `(table_hash, record_id)` pair.
    pub fn get(&self, table_hash: u32, record_id: i32) -> Option<&[u8]> {
        let table = self.blobs.get(&table_hash)?;
        table.get(&(record_id as u32)).map(|v| v.as_slice())
    }

    /// Whether the cache has any data for a given table hash.
    pub fn has_table(&self, table_hash: u32) -> bool {
        self.blobs.contains_key(&table_hash)
    }

    /// Total number of blobs cached across all tables.
    pub fn total_blobs(&self) -> usize {
        self.blobs.values().map(|t| t.len()).sum()
    }
}

/// Helper: load `Item.db2` + `ItemSparse.db2` (and any other needed files) and log progress.
pub fn build_hotfix_blob_cache(data_dir: &str, locale: &str) -> HotfixBlobCache {
    let mut cache = HotfixBlobCache::new();

    let dbc_dir = Path::new(data_dir).join("dbc").join(locale);

    // Load Item.db2 — the client needs this alongside ItemSparse to display item info
    let item_db2 = dbc_dir.join("Item.db2");
    if item_db2.exists() {
        match cache.load_db2(&item_db2) {
            Ok(n) => info!("HotfixBlobCache: loaded {} Item records", n),
            Err(e) => tracing::warn!("HotfixBlobCache: failed to load Item.db2: {e}"),
        }
    } else {
        tracing::warn!("HotfixBlobCache: Item.db2 not found at {}", item_db2.display());
    }

    // Load ItemSparse.db2
    let item_sparse = dbc_dir.join("ItemSparse.db2");

    if item_sparse.exists() {
        match cache.load_db2(&item_sparse) {
            Ok(n) => info!("HotfixBlobCache: loaded {} ItemSparse records", n),
            Err(e) => tracing::warn!("HotfixBlobCache: failed to load ItemSparse.db2: {e}"),
        }
    } else {
        tracing::warn!("HotfixBlobCache: ItemSparse.db2 not found at {}", item_sparse.display());
    }

    info!(
        "HotfixBlobCache: {} total blobs cached",
        cache.total_blobs()
    );
    cache
}

