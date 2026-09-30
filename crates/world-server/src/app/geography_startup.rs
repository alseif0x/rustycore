//! Ordered geography, phasing and graveyard startup catalogs.

use std::sync::Arc;

use anyhow::Context;
use tracing::info;

use crate::catalogs;

pub(super) struct GeographyBase {
    pub(super) area_trigger_db2_store: Arc<wow_data::AreaTriggerDb2Store>,
    pub(super) area_table_store: Arc<wow_data::AreaTableStore>,
    pub(super) ui_map_x_map_art_store: Arc<wow_data::UiMapXMapArtStore>,
    pub(super) world_safe_loc_report: wow_data::WorldSafeLocLoadReport,
    pub(super) world_safe_loc_store: Arc<wow_data::WorldSafeLocStore>,
    pub(super) map_store: Arc<wow_data::MapStore>,
}

pub(super) async fn load_geography_base(
    data_dir: &str,
    locale: &str,
    world_reference_catalog_persistence: &dyn wow_persistence::WorldReferenceCatalogPersistencePortLikeCpp,
    static_data_overlay_persistence: &dyn wow_persistence::StaticDataOverlayPersistencePortLikeCpp,
) -> anyhow::Result<GeographyBase> {
    // Load Map.db2 + MapDifficulty.db2 for C++ InstanceLockMgr MapDb2Entries resolution.
    let map_store = Arc::new(
        wow_data::MapStore::load(data_dir, locale)
            .context("Failed to load Map.db2 — check DataDir and DBC.Locale config")?,
    );
    info!("Loaded {} maps from Map.db2", map_store.len());
    let (world_safe_loc_store, world_safe_loc_report) =
        crate::world::reference_catalog::load_world_safe_locs_like_cpp(
            world_reference_catalog_persistence,
            &map_store,
        )
        .await
        .context("Failed to load C++ world_safe_locs")?;
    info!(
        "Loaded {} world safe locs ({} missing maps, {} invalid positions)",
        world_safe_loc_store.len(),
        world_safe_loc_report.missing_maps.len(),
        world_safe_loc_report.invalid_positions.len()
    );
    let world_safe_loc_store = Arc::new(world_safe_loc_store);
    let ui_map_x_map_art_store = Arc::new(
        crate::static_data_overlay::load_ui_map_x_map_art_store_like_cpp(
            data_dir,
            locale,
            static_data_overlay_persistence,
        )
        .await
        .context("Failed to load UiMapXMapArt.db2 / hotfix rows")?,
    );
    let area_table_store = Arc::new(
        crate::static_data_overlay::load_area_table_store_like_cpp(
            data_dir,
            locale,
            static_data_overlay_persistence,
        )
        .await
        .context("Failed to load AreaTable.db2 / hotfix rows")?,
    );
    let area_trigger_db2_store = Arc::new(
        wow_data::AreaTriggerDb2Store::load(data_dir, locale)
            .context("Failed to load AreaTrigger.db2 — check DataDir and DBC.Locale config")?,
    );
    info!(
        "Loaded {} area trigger DB2 rows from AreaTrigger.db2",
        area_trigger_db2_store.len()
    );
    Ok(GeographyBase {
        map_store,
        world_safe_loc_store,
        world_safe_loc_report,
        ui_map_x_map_art_store,
        area_table_store,
        area_trigger_db2_store,
    })
}

pub(super) struct PhaseAndGraveyardCatalogs {
    pub(super) phase_hotfix_adapter: wow_database::MariaDbPhaseHotfixPersistenceAdapterLikeCpp,
    pub(super) phase_store: Arc<wow_data::PhaseStore>,
    pub(super) phase_group_store: Arc<wow_data::PhaseGroupStore>,
    pub(super) phase_world_adapter: wow_database::MariaDbPhaseWorldCatalogPersistenceAdapterLikeCpp,
    pub(super) phase_info_store: wow_data::PhaseInfoStore,
    pub(super) _phase_name_store: Arc<wow_data::PhaseNameStoreLikeCpp>,
    pub(super) terrain_swap_store: Arc<wow_data::TerrainSwapStore>,
    pub(super) graveyard_store: wow_data::GraveyardStore,
    pub(super) graveyard_report: wow_data::GraveyardLoadReport,
}

pub(super) async fn load_phase_and_graveyard_catalogs(
    data_dir: &str,
    locale: &str,
    hotfix_db: &Arc<wow_database::HotfixDatabase>,
    world_db: &Arc<wow_database::WorldDatabase>,
    map_store: &wow_data::MapStore,
    area_table_store: &wow_data::AreaTableStore,
    ui_map_x_map_art_store: &wow_data::UiMapXMapArtStore,
    world_safe_loc_store: &wow_data::WorldSafeLocStore,
    world_auxiliary_catalog_persistence: &dyn wow_persistence::WorldAuxiliaryCatalogPersistencePortLikeCpp,
) -> anyhow::Result<PhaseAndGraveyardCatalogs> {
    let phase_hotfix_adapter =
        wow_database::MariaDbPhaseHotfixPersistenceAdapterLikeCpp::new(Arc::clone(hotfix_db));
    let (phase_store, phase_group_store) = catalogs::phase_hotfix::load_phase_stores_like_cpp(
        data_dir,
        locale,
        &phase_hotfix_adapter,
    )
    .await
    .context("Failed to load Phase/PhaseXPhaseGroup DB2 and hotfix rows")?;
    let phase_store = Arc::new(phase_store);
    let phase_group_store = Arc::new(phase_group_store);
    info!(
        "Loaded {} phases and {} phase-group rows",
        phase_store.len(),
        phase_group_store.len()
    );
    let phase_world_adapter =
        wow_database::MariaDbPhaseWorldCatalogPersistenceAdapterLikeCpp::new(Arc::clone(world_db));
    let (phase_info_store, phase_name_store, terrain_swap_store) =
        catalogs::phase_world::load_phase_world_catalogs_like_cpp(
            &phase_world_adapter,
            area_table_store,
            &phase_store,
            map_store,
            |phase_id| ui_map_x_map_art_store.is_ui_map_phase(phase_id),
        )
        .await?;
    let _phase_name_store = Arc::new(phase_name_store);
    let terrain_swap_store = Arc::new(terrain_swap_store);
    let mut graveyard_store = wow_data::GraveyardStore::default();
    let graveyard_report = crate::world::auxiliary_catalog::load_graveyard_zones_like_cpp(
        world_auxiliary_catalog_persistence,
        &mut graveyard_store,
        |safe_loc_id| world_safe_loc_store.contains(safe_loc_id),
        |area_id| area_table_store.get(area_id).is_some(),
    )
    .await
    .context("Failed to load C++ graveyard_zone links")?;
    info!(
        "Loaded {} graveyard-zone links ({} missing safe locs, {} missing zones, {} duplicates)",
        graveyard_report.loaded,
        graveyard_report.missing_safe_locs.len(),
        graveyard_report.missing_zones.len(),
        graveyard_report.duplicates.len()
    );
    Ok(PhaseAndGraveyardCatalogs {
        phase_hotfix_adapter,
        phase_store,
        phase_group_store,
        phase_world_adapter,
        phase_info_store,
        _phase_name_store,
        terrain_swap_store,
        graveyard_store,
        graveyard_report,
    })
}

pub(super) fn load_map_difficulty_catalogs(
    data_dir: &str,
    locale: &str,
) -> anyhow::Result<(
    Arc<wow_data::MapDifficultyStore>,
    Arc<wow_data::MapDifficultyXConditionStore>,
)> {
    let map_difficulty_store = Arc::new(
        wow_data::MapDifficultyStore::load(data_dir, locale)
            .context("Failed to load MapDifficulty.db2 — check DataDir and DBC.Locale config")?,
    );
    info!(
        "Loaded {} map difficulties from MapDifficulty.db2",
        map_difficulty_store.len()
    );
    let map_difficulty_x_condition_store = Arc::new(
        wow_data::MapDifficultyXConditionStore::load(data_dir, locale).context(
            "Failed to load MapDifficultyXCondition.db2 — check DataDir and DBC.Locale config",
        )?,
    );
    info!(
        "Loaded {} map difficulty conditions from MapDifficultyXCondition.db2",
        map_difficulty_x_condition_store.len()
    );
    Ok((map_difficulty_store, map_difficulty_x_condition_store))
}
