//! Battle-pet stat and combat game-table catalogs produced during ordered startup.

use super::*;

pub(super) struct BattlePetStatCatalogs {
    pub(super) battle_pet_breed_quality_store: Arc<wow_data::BattlePetBreedQualityStore>,
    pub(super) battle_pet_breed_state_store: Arc<wow_data::BattlePetBreedStateStore>,
    pub(super) battle_pet_species_entry_store: Arc<wow_data::BattlePetSpeciesStore>,
    pub(super) battle_pet_species_state_store: Arc<wow_data::BattlePetSpeciesStateStore>,
    pub(super) battle_pet_xp_game_table: Arc<wow_data::BattlePetXpGameTableLikeCpp>,
    pub(super) combat_ratings_game_table: Arc<wow_data::CombatRatingsGameTableLikeCpp>,
    pub(super) shield_block_regular_game_table: Arc<wow_data::ShieldBlockRegularGameTableLikeCpp>,
}

pub(super) fn load_battle_pet_stat_catalogs(
    data_dir: &str,
    locale: &str,
) -> Result<BattlePetStatCatalogs> {
    // Load battle-pet stat DB2 stores used by BattlePet::CalculateStats.
    let battle_pet_breed_quality_store = Arc::new(
        wow_data::BattlePetBreedQualityStore::load(&data_dir, &locale).context(
            "Failed to load BattlePetBreedQuality.db2 — check DataDir and DBC.Locale config",
        )?,
    );
    let battle_pet_breed_state_store = Arc::new(
        wow_data::BattlePetBreedStateStore::load(&data_dir, &locale).context(
            "Failed to load BattlePetBreedState.db2 — check DataDir and DBC.Locale config",
        )?,
    );
    let battle_pet_species_entry_store = Arc::new(
        wow_data::BattlePetSpeciesStore::load(&data_dir, &locale)
            .context("Failed to load BattlePetSpecies.db2 — check DataDir and DBC.Locale config")?,
    );
    let battle_pet_species_state_store = Arc::new(
        wow_data::BattlePetSpeciesStateStore::load(&data_dir, &locale).context(
            "Failed to load BattlePetSpeciesState.db2 — check DataDir and DBC.Locale config",
        )?,
    );
    let battle_pet_xp_game_table = Arc::new(
        wow_data::BattlePetXpGameTableLikeCpp::load(&data_dir)
            .context("Failed to load gt/BattlePetXP.txt — check DataDir config")?,
    );
    let combat_ratings_game_table = Arc::new(
        wow_data::CombatRatingsGameTableLikeCpp::load(&data_dir)
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
        wow_data::ShieldBlockRegularGameTableLikeCpp::load(&data_dir)
            .context("Failed to load gt/ShieldBlockRegular.txt - check DataDir config")?,
    );
    info!(
        "Loaded ShieldBlockRegular game table: {} rows",
        shield_block_regular_game_table.len()
    );

    Ok(BattlePetStatCatalogs {
        battle_pet_breed_quality_store,
        battle_pet_breed_state_store,
        battle_pet_species_entry_store,
        battle_pet_species_state_store,
        battle_pet_xp_game_table,
        combat_ratings_game_table,
        shield_block_regular_game_table,
    })
}
