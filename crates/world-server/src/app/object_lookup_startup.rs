//! Ordered object lookup startup composition.

use anyhow::Context;
use std::sync::Arc;
use tracing::info;
use crate::catalogs;

pub(super) struct ObjectLookupCatalogs {
    pub(super) game_tele_report: wow_data::GameTeleLoadReportLikeCpp,
    pub(super) game_tele_store: Arc<wow_data::GameTeleStoreLikeCpp>,
    pub(super) game_tele_persistence: wow_database::MariaDbGameTeleCatalogPersistenceAdapterLikeCpp,
    pub(super) reserved_name_store: Arc<wow_data::ReservedNameStoreLikeCpp>,
    pub(super) reserved_name_persistence: wow_database::MariaDbReservedNameCatalogPersistenceAdapterLikeCpp,
}

pub(super) async fn load(
    char_db: &Arc<wow_database::CharacterDatabase>,
    world_db: &Arc<wow_database::WorldDatabase>,
) -> anyhow::Result<ObjectLookupCatalogs> {
    let reserved_name_persistence =
        wow_database::MariaDbReservedNameCatalogPersistenceAdapterLikeCpp::new(Arc::clone(
            &char_db,
        ));
    let reserved_name_store = Arc::new(
        catalogs::reserved_name::load_reserved_name_catalog_like_cpp(&reserved_name_persistence)
            .await
            .context("Failed to load C++ reserved player names")?,
    );
    info!(
        "Loaded {} C++ reserved player names ({} unique)",
        reserved_name_store.loaded_rows_like_cpp(),
        reserved_name_store.len()
    );
    let game_tele_persistence =
        wow_database::MariaDbGameTeleCatalogPersistenceAdapterLikeCpp::new(Arc::clone(world_db));
    let game_tele_outcome =
        catalogs::game_tele::load_game_tele_catalog_like_cpp(&game_tele_persistence)
            .await
            .context("Failed to load C++ game teleport locations")?;
    for (id, name) in &game_tele_outcome.report.skipped_invalid_coordinates {
        tracing::error!(
            "Wrong position for id {} (name: {}) in `game_tele` table, ignoring.",
            id,
            name
        );
    }
    let game_tele_store = Arc::new(game_tele_outcome.store);
    info!(
        "Loaded {} C++ GameTeleports ({} unique ids)",
        game_tele_outcome.report.loaded_rows,
        game_tele_store.len()
    );
    Ok(ObjectLookupCatalogs {
        reserved_name_persistence,
        reserved_name_store,
        game_tele_persistence,
        game_tele_store,
        game_tele_report: game_tele_outcome.report,
    })
}

pub(super) async fn load_localized_strings(
    world_auxiliary_catalog_persistence: &dyn wow_persistence::WorldAuxiliaryCatalogPersistencePortLikeCpp,
) -> anyhow::Result<Arc<wow_data::TrinityStringStoreLikeCpp>> {
    let trinity_string_store = Arc::new(
        crate::world::auxiliary_catalog::load_trinity_strings_like_cpp(
            world_auxiliary_catalog_persistence,
        )
        .await
        .context("Failed to load C++ trinity_string rows")?,
    );
    info!(
        "Loaded {} C++ trinity_string rows",
        trinity_string_store.len()
    );
    Ok(trinity_string_store)
}
