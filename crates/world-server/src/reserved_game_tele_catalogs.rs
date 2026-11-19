//! Reserved-name and game-teleport catalogs produced during ordered startup.

use super::*;

pub(super) struct ReservedGameTeleCatalogs {
    pub(super) _reserved_name_store: Arc<wow_data::ReservedNameStoreLikeCpp>,
    pub(super) _game_tele_store: Arc<wow_data::GameTeleStoreLikeCpp>,
}

pub(super) async fn load_reserved_game_tele_catalogs(
    char_db: &Arc<CharacterDatabase>,
    world_db: &Arc<WorldDatabase>,
) -> Result<ReservedGameTeleCatalogs> {
    let reserved_name_persistence =
        wow_database::MariaDbReservedNameCatalogPersistenceAdapterLikeCpp::new(Arc::clone(char_db));
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

    Ok(ReservedGameTeleCatalogs {
        _reserved_name_store: reserved_name_store,
        _game_tele_store: game_tele_store,
    })
}
