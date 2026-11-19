//! AreaTrigger world, script and tavern catalogs produced during ordered startup.

use super::*;

pub(super) struct AreaTriggerWorldStartupCatalogs {
    pub(super) area_trigger_store: Arc<wow_data::AreaTriggerStore>,
    pub(super) area_trigger_script_store: Arc<wow_data::AreaTriggerScriptStoreLikeCpp>,
    pub(super) tavern_area_trigger_store: Arc<wow_data::TavernAreaTriggerStoreLikeCpp>,
}

pub(super) async fn load_area_trigger_world_startup_catalogs(
    world_db: &Arc<WorldDatabase>,
    area_trigger_db2_store: &wow_data::AreaTriggerDb2Store,
    script_name_interner: &mut Arc<wow_data::ScriptNameInternerLikeCpp>,
) -> Result<AreaTriggerWorldStartupCatalogs> {
    // Load area trigger store (collision detection + teleportation)
    let area_trigger_world_persistence =
        wow_database::MariaDbAreaTriggerWorldCatalogPersistenceAdapterLikeCpp::new(Arc::clone(
            world_db,
        ));
    let area_trigger_world_catalogs =
        crate::area::trigger_world_catalog::load_area_trigger_world_catalogs_like_cpp(
            &area_trigger_world_persistence,
            area_trigger_db2_store,
            Arc::make_mut(script_name_interner),
        )
        .await?;
    let area_trigger_store = area_trigger_world_catalogs.area_trigger_store;
    let area_trigger_script_outcome = area_trigger_world_catalogs.script_outcome;
    let area_trigger_script_store = Arc::new(area_trigger_script_outcome.store);
    info!(
        "Loaded {} C++ area trigger script bindings ({} skipped missing area trigger)",
        area_trigger_script_store.len(),
        area_trigger_script_outcome
            .report
            .skipped_missing_area_trigger
            .len()
    );
    let tavern_area_trigger_outcome = area_trigger_world_catalogs.tavern_outcome;
    let tavern_area_trigger_store = Arc::new(tavern_area_trigger_outcome.store);
    info!(
        "Loaded {} C++ tavern area triggers ({} rows seen; {} skipped missing AreaTrigger.db2)",
        tavern_area_trigger_store.len(),
        tavern_area_trigger_outcome.report.rows_seen,
        tavern_area_trigger_outcome
            .report
            .skipped_missing_area_trigger
            .len()
    );

    Ok(AreaTriggerWorldStartupCatalogs {
        area_trigger_store,
        area_trigger_script_store,
        tavern_area_trigger_store,
    })
}
