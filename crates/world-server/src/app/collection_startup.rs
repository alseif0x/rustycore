//! Mount, heirloom and toy collection authority startup.

use std::sync::Arc;
use anyhow::Context;
use tracing::info;
use crate::catalogs;

pub(super) struct CollectionCatalogs {
    pub(super) toy_store: Arc<wow_data::ToyStore>,
    pub(super) heirloom_store: Arc<wow_data::HeirloomStore>,
    pub(super) mount_x_display_store: Arc<wow_data::MountXDisplayStore>,
    pub(super) mount_type_x_capability_store: Arc<wow_data::MountTypeXCapabilityStore>,
    pub(super) mount_capability_store: Arc<wow_data::MountCapabilityStore>,
    pub(super) mount_definition_store: Arc<wow_data::MountDefinitionStoreLikeCpp>,
    pub(super) mount_store: Arc<wow_data::MountStore>,
    pub(super) mount_catalog_persistence: wow_database::MariaDbMountCatalogPersistenceAdapterLikeCpp,
}

pub(super) async fn load(data_dir: &str, locale: &str, hotfix_db: &Arc<wow_database::HotfixDatabase>, world_db: &Arc<wow_database::WorldDatabase>) -> anyhow::Result<CollectionCatalogs> {
    let mount_catalog_persistence = wow_database::MariaDbMountCatalogPersistenceAdapterLikeCpp::new(
        Arc::clone(hotfix_db),
        Arc::clone(world_db),
    );
    let (mount_store, mount_hotfix_rows) =
        catalogs::mount::load_mount_store_like_cpp(data_dir, locale, &mount_catalog_persistence)
            .await
            .context("Failed to load Mount.db2 / hotfix rows")?;
    if mount_hotfix_rows != 0 {
        info!("Loaded {mount_hotfix_rows} Mount hotfix rows");
    }
    let mount_store = Arc::new(mount_store);
    info!("Loaded {} mounts from Mount.db2", mount_store.len());
    let mount_definition_store = Arc::new(
        catalogs::mount::load_mount_definition_store_like_cpp(
            &mount_store,
            &mount_catalog_persistence,
        )
        .await
        .context("Failed to load mount_definitions")?,
    );
    info!(
        "Loaded {} faction-specific mount definitions from mount_definitions",
        mount_definition_store.len()
    );
    let (mount_capability_store, mount_capability_hotfix_rows) =
        catalogs::mount::load_mount_capability_store_like_cpp(
            data_dir,
            locale,
            &mount_catalog_persistence,
        )
        .await
        .context("Failed to load MountCapability.db2 / hotfix rows")?;
    if mount_capability_hotfix_rows != 0 {
        info!("Loaded {mount_capability_hotfix_rows} MountCapability hotfix rows");
    }
    let mount_capability_store = Arc::new(mount_capability_store);
    info!(
        "Loaded {} mount capabilities from MountCapability.db2",
        mount_capability_store.len()
    );
    let (mount_type_x_capability_store, mount_type_x_capability_hotfix_rows) =
        catalogs::mount::load_mount_type_x_capability_store_like_cpp(
            data_dir,
            locale,
            &mount_catalog_persistence,
        )
        .await
        .context("Failed to load MountTypeXCapability.db2 / hotfix rows")?;
    if mount_type_x_capability_hotfix_rows != 0 {
        info!("Loaded {mount_type_x_capability_hotfix_rows} MountTypeXCapability hotfix rows");
    }
    let mount_type_x_capability_store = Arc::new(mount_type_x_capability_store);
    info!(
        "Loaded {} mount type capability rows from MountTypeXCapability.db2",
        mount_type_x_capability_store.len()
    );
    let (mount_x_display_store, mount_x_display_hotfix_rows) =
        catalogs::mount::load_mount_x_display_store_like_cpp(
            data_dir,
            locale,
            &mount_catalog_persistence,
        )
        .await
        .context("Failed to load MountXDisplay.db2 / hotfix rows")?;
    if mount_x_display_hotfix_rows != 0 {
        info!("Loaded {mount_x_display_hotfix_rows} MountXDisplay hotfix rows");
    }
    let mount_x_display_store = Arc::new(mount_x_display_store);
    info!(
        "Loaded {} mount display rows from MountXDisplay.db2",
        mount_x_display_store.len()
    );
    let heirloom_store = Arc::new(
        wow_data::HeirloomStore::load(data_dir, locale)
            .context("Failed to load Heirloom.db2 — check DataDir and DBC.Locale config")?,
    );
    info!(
        "Loaded {} heirlooms from Heirloom.db2",
        heirloom_store.len()
    );
    let toy_store = Arc::new(
        wow_data::ToyStore::load(data_dir, locale)
            .context("Failed to load Toy.db2 — check DataDir and DBC.Locale config")?,
    );
    info!("Loaded {} toys from Toy.db2", toy_store.len());
    Ok(CollectionCatalogs {
        mount_catalog_persistence,
        mount_store,
        mount_definition_store,
        mount_capability_store,
        mount_type_x_capability_store,
        mount_x_display_store,
        heirloom_store,
        toy_store,
    })
}

pub(super) struct TransmogCatalogs {
    pub(super) transmog_set_item_store: Arc<wow_data::TransmogSetItemStore>,
    pub(super) transmog_set_store: Arc<wow_data::TransmogSetStore>,
}

pub(super) fn load_transmogs(
    data_dir: &str,
    locale: &str,
) -> anyhow::Result<TransmogCatalogs> {
    // Load TransmogSet.db2 and TransmogSetItem.db2 for DB2Manager transmog indexes.
    let transmog_set_store = Arc::new(
        wow_data::TransmogSetStore::load(data_dir, locale)
            .context("Failed to load TransmogSet.db2 — check DataDir and DBC.Locale config")?,
    );
    info!(
        "Loaded {} transmog sets from TransmogSet.db2",
        transmog_set_store.len()
    );
    let transmog_set_item_store = Arc::new(
        wow_data::TransmogSetItemStore::load_with_sets(data_dir, locale, &transmog_set_store)
            .context("Failed to load TransmogSetItem.db2 — check DataDir and DBC.Locale config")?,
    );
    info!(
        "Loaded {} transmog set items from TransmogSetItem.db2",
        transmog_set_item_store.len()
    );
    Ok(TransmogCatalogs {
        transmog_set_store,
        transmog_set_item_store,
    })
}

pub(super) fn load_item_search_names(
    data_dir: &str,
    locale: &str,
) -> anyhow::Result<Arc<wow_data::ItemSearchNameStore>> {
    // Load ItemSearchName.db2 for CollectionMgr::CanAddAppearance item-name existence gate.
    let item_search_name_store = Arc::new(
        wow_data::ItemSearchNameStore::load(data_dir, locale)
            .context("Failed to load ItemSearchName.db2 — check DataDir and DBC.Locale config")?,
    );
    info!(
        "Loaded {} item search-name rows from ItemSearchName.db2",
        item_search_name_store.len()
    );
    Ok(item_search_name_store)
}
