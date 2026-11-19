//! Mount catalogs produced during ordered startup.

use super::*;

pub(super) struct MountCatalogs {
    pub(super) mount_store: Arc<wow_data::MountStore>,
    pub(super) mount_definition_store: Arc<wow_data::MountDefinitionStoreLikeCpp>,
    pub(super) mount_capability_store: Arc<wow_data::MountCapabilityStore>,
    pub(super) mount_type_x_capability_store: Arc<wow_data::MountTypeXCapabilityStore>,
    pub(super) mount_x_display_store: Arc<wow_data::MountXDisplayStore>,
}

pub(super) async fn load_mount_catalogs(
    data_dir: &str,
    locale: &str,
    hotfix_db: &Arc<HotfixDatabase>,
    world_db: &Arc<WorldDatabase>,
) -> Result<MountCatalogs> {
    let mount_catalog_persistence = wow_database::MariaDbMountCatalogPersistenceAdapterLikeCpp::new(
        Arc::clone(hotfix_db),
        Arc::clone(world_db),
    );
    let (mount_store, mount_hotfix_rows) =
        catalogs::mount::load_mount_store_like_cpp(&data_dir, &locale, &mount_catalog_persistence)
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
            &data_dir,
            &locale,
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
            &data_dir,
            &locale,
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
            &data_dir,
            &locale,
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
    Ok(MountCatalogs {
        mount_store,
        mount_definition_store,
        mount_capability_store,
        mount_type_x_capability_store,
        mount_x_display_store,
    })
}
