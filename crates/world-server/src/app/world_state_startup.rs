//! Condition world IDs and canonical WorldState startup composition.

use std::sync::{Arc, Mutex};

use anyhow::Context;
use tracing::info;

use crate::{spawn_store_loader, SharedWorldStateMgrLikeCpp};

pub(super) async fn load_condition_world_ids(
    world_reference_catalog_persistence: &dyn wow_persistence::WorldReferenceCatalogPersistencePortLikeCpp,
) -> anyhow::Result<(Arc<wow_data::WorldIdStore>, Arc<wow_data::WorldIdStore>)> {
    let active_event_store = Arc::new(
        crate::world::reference_catalog::load_world_id_store_like_cpp(
            world_reference_catalog_persistence,
            wow_persistence::WorldObjectIdCatalogKindLikeCpp::GameEvent,
        )
        .await
        .context("Failed to load game_event ids for C++ ConditionMgr validation")?,
    );
    let world_state_store = Arc::new(
        crate::world::reference_catalog::load_world_id_store_like_cpp(
            world_reference_catalog_persistence,
            wow_persistence::WorldObjectIdCatalogKindLikeCpp::WorldState,
        )
        .await
        .context("Failed to load world_state ids for C++ ConditionMgr validation")?,
    );
    info!(
        "Loaded condition validation world id stores: {} valid game events, {} world states",
        active_event_store.len(),
        world_state_store.len()
    );
    Ok((active_event_store, world_state_store))
}

pub(super) async fn load_world_state_startup(
    world_db: &Arc<wow_database::WorldDatabase>,
    char_db: &Arc<wow_database::CharacterDatabase>,
    map_store: &wow_data::MapStore,
    area_table_store: &wow_data::AreaTableStore,
) -> anyhow::Result<(
    SharedWorldStateMgrLikeCpp,
    Arc<dyn wow_persistence::WorldStateStartupPersistencePortLikeCpp>,
    spawn_store_loader::WorldStateMgrLoadReportLikeCpp,
)> {
    let world_state_startup: Arc<dyn wow_persistence::WorldStateStartupPersistencePortLikeCpp> =
        Arc::new(
            wow_database::MariaDbWorldStateStartupPersistenceAdapterLikeCpp::new(
                Arc::clone(world_db),
                Arc::clone(char_db),
            ),
        );
    let (world_state_mgr, world_state_mgr_report) =
        spawn_store_loader::load_world_state_mgr_like_cpp(
            world_state_startup.as_ref(),
            map_store,
            area_table_store,
        )
        .await
        .context("Failed to load C++ WorldStateMgr startup state")?;
    info!(
        "Loaded C++ WorldStateMgr startup state: template rows={} loaded={} skipped-map-list={} skipped-area-list={} realm-area-ignored={} saved rows={} applied={} skipped-unknown={}",
        world_state_mgr_report.template_rows,
        world_state_mgr_report.templates_loaded,
        world_state_mgr_report.skipped_invalid_map_list,
        world_state_mgr_report.skipped_invalid_area_list,
        world_state_mgr_report.realm_area_requirements_ignored,
        world_state_mgr_report.saved_rows,
        world_state_mgr_report.saved_applied,
        world_state_mgr_report.saved_skipped_unknown,
    );
    let world_state_mgr: SharedWorldStateMgrLikeCpp = Arc::new(Mutex::new(world_state_mgr));
    Ok((world_state_mgr, world_state_startup, world_state_mgr_report))
}
