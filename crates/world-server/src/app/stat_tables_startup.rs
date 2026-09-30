//! Battle-pet stat authority and player combat game tables.

use anyhow::Context;
use std::sync::Arc;
use tracing::info;

pub(super) struct StatTables {
    pub(super) shield_block_regular_game_table: Arc<wow_data::ShieldBlockRegularGameTableLikeCpp>,
    pub(super) combat_ratings_game_table: Arc<wow_data::CombatRatingsGameTableLikeCpp>,
    pub(super) battle_pet_xp_game_table: Arc<wow_data::BattlePetXpGameTableLikeCpp>,
    pub(super) battle_pet_species_state_store: Arc<wow_data::BattlePetSpeciesStateStore>,
    pub(super) battle_pet_species_entry_store: Arc<wow_data::BattlePetSpeciesStore>,
    pub(super) battle_pet_breed_state_store: Arc<wow_data::BattlePetBreedStateStore>,
    pub(super) battle_pet_breed_quality_store: Arc<wow_data::BattlePetBreedQualityStore>,
}

pub(super) fn load(data_dir: &str, locale: &str) -> anyhow::Result<StatTables> {
    // Load battle-pet stat DB2 stores used by BattlePet::CalculateStats.
    let battle_pet_breed_quality_store = Arc::new(
        wow_data::BattlePetBreedQualityStore::load(data_dir, locale).context(
            "Failed to load BattlePetBreedQuality.db2 — check DataDir and DBC.Locale config",
        )?,
    );
    let battle_pet_breed_state_store = Arc::new(
        wow_data::BattlePetBreedStateStore::load(data_dir, locale).context(
            "Failed to load BattlePetBreedState.db2 — check DataDir and DBC.Locale config",
        )?,
    );
    let battle_pet_species_entry_store = Arc::new(
        wow_data::BattlePetSpeciesStore::load(data_dir, locale)
            .context("Failed to load BattlePetSpecies.db2 — check DataDir and DBC.Locale config")?,
    );
    let battle_pet_species_state_store = Arc::new(
        wow_data::BattlePetSpeciesStateStore::load(data_dir, locale).context(
            "Failed to load BattlePetSpeciesState.db2 — check DataDir and DBC.Locale config",
        )?,
    );
    let battle_pet_xp_game_table = Arc::new(
        wow_data::BattlePetXpGameTableLikeCpp::load(data_dir)
            .context("Failed to load gt/BattlePetXP.txt — check DataDir config")?,
    );
    let combat_ratings_game_table = Arc::new(
        wow_data::CombatRatingsGameTableLikeCpp::load(data_dir)
            .context("Failed to load gt/CombatRatings.txt - check DataDir config")?,
    );
    info!(
        "Loaded battle-pet stat DB2 stores: {} quality rows, {} breed-state rows, {} species rows, {} species-state rows; BattlePetXP rows={}; CombatRatings rows={}",
        battle_pet_breed_quality_store.len(),
        battle_pet_breed_state_store.len(),
        battle_pet_species_entry_store.len(),
        battle_pet_species_state_store.len(),
        battle_pet_xp_game_table.len(),
        combat_ratings_game_table.len()
    );
    let shield_block_regular_game_table = Arc::new(
        wow_data::ShieldBlockRegularGameTableLikeCpp::load(data_dir)
            .context("Failed to load gt/ShieldBlockRegular.txt - check DataDir config")?,
    );
    info!(
        "Loaded ShieldBlockRegular game table: {} rows",
        shield_block_regular_game_table.len()
    );
    Ok(StatTables {
        battle_pet_breed_quality_store,
        battle_pet_breed_state_store,
        battle_pet_species_entry_store,
        battle_pet_species_state_store,
        battle_pet_xp_game_table,
        combat_ratings_game_table,
        shield_block_regular_game_table,
    })
}

pub(super) struct BattlePetSelection {
    pub(super) battle_pet_selection_store:
        Arc<wow_data::battle_pet_selection::BattlePetSelectionStoreLikeCpp>,
    pub(super) battle_pet_selection_persistence:
        wow_database::MariaDbBattlePetSelectionCatalogPersistenceAdapterLikeCpp,
}

pub(super) async fn load_battle_pet_selection(
    world_db: &Arc<wow_database::WorldDatabase>,
    battle_pet_species_entry_store: &wow_data::BattlePetSpeciesStore,
) -> anyhow::Result<BattlePetSelection> {
    // C++ loads breeds and qualities independently and tolerates either
    // unavailable table as an empty catalog.
    let battle_pet_selection_persistence =
        wow_database::MariaDbBattlePetSelectionCatalogPersistenceAdapterLikeCpp::new(Arc::clone(
            &world_db,
        ));
    let battle_pet_selection_store = Arc::new(
        crate::catalogs::battle_pet_selection::load_battle_pet_selection_store_like_cpp(
            &battle_pet_selection_persistence,
            |species| {
                battle_pet_species_entry_store
                    .get(species)
                    .map(|entry| entry.flags)
            },
        )
        .await,
    );
    info!(
        "Loaded {} battle-pet breed/quality selection rows",
        battle_pet_selection_store.len_like_cpp()
    );
    Ok(BattlePetSelection {
        battle_pet_selection_persistence,
        battle_pet_selection_store,
    })
}
