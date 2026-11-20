//! Ordered hotfix delivery metadata and TactKey startup.

use super::*;

pub(super) struct HotfixDeliveryStartup {
    pub(super) hotfix_blob_cache: Arc<wow_data::HotfixBlobCache>,
    pub(super) tact_key_store: Arc<wow_data::TactKeyStore>,
}

pub(super) async fn load_hotfix_delivery_startup(
    data_dir: &str,
    locale: &str,
    persistence: &wow_database::MariaDbHotfixDeliveryMetadataPersistenceAdapterLikeCpp,
) -> Result<HotfixDeliveryStartup> {
    // Build hotfix blob cache — pre-loads raw DB2 record bytes and hotfix DB overlays for DBReply.
    let mut hotfix_blob_cache = wow_data::build_hotfix_blob_cache(data_dir, locale);
    let [hotfix_blobs, hotfix_data, hotfix_optional_data] =
        crate::hotfix_delivery_metadata::load_hotfix_delivery_metadata_like_cpp(
            &mut hotfix_blob_cache,
            persistence,
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

    Ok(HotfixDeliveryStartup {
        hotfix_blob_cache,
        tact_key_store,
    })
}
