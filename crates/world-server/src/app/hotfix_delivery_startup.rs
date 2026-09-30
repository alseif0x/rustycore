//! Ordered hotfix reply metadata and TactKey startup.

use std::sync::Arc;

use anyhow::{Context, Result};
use tracing::info;

pub(super) async fn load(
    data_dir: &str,
    locale: &str,
    hotfix_delivery_metadata_persistence: &dyn wow_persistence::HotfixDeliveryMetadataPersistencePortLikeCpp,
) -> Result<(Arc<wow_data::HotfixBlobCache>, Arc<wow_data::TactKeyStore>)> {
    // Build hotfix blob cache — pre-loads raw DB2 record bytes and hotfix DB overlays for DBReply.
    let mut hotfix_blob_cache = wow_data::build_hotfix_blob_cache(data_dir, locale);
    let [hotfix_blobs, hotfix_data, hotfix_optional_data] =
        crate::hotfix_delivery_metadata::load_hotfix_delivery_metadata_like_cpp(
            &mut hotfix_blob_cache,
            hotfix_delivery_metadata_persistence,
            locale,
        )
        .await;
    match hotfix_blobs {
        Ok(n) => info!("HotfixBlobCache: loaded {n} hotfix_blob rows"),
        Err(e) => tracing::warn!("HotfixBlobCache: failed to load hotfix_blob rows: {e}"),
    }
    match hotfix_data {
        Ok(n) => info!("HotfixBlobCache: loaded {n} hotfix_data rows"),
        Err(e) => tracing::warn!("HotfixBlobCache: failed to load hotfix_data rows: {e}"),
    }
    match hotfix_optional_data {
        Ok(n) => info!("HotfixBlobCache: loaded {n} hotfix_optional_data rows"),
        Err(e) => tracing::warn!("HotfixBlobCache: failed to load hotfix_optional_data rows: {e}"),
    }
    let hotfix_blob_cache = Arc::new(hotfix_blob_cache);
    let tact_key_store = Arc::new(
        wow_data::TactKeyStore::load(data_dir, locale).context("Failed to load TactKey.db2")?,
    );
    info!(
        "Loaded {} TactKey rows from TactKey.db2",
        tact_key_store.len()
    );
    Ok((hotfix_blob_cache, tact_key_store))
}
