use super::world_config_u8;
use anyhow::Context;
use std::sync::Arc;
use tracing::info;

pub(super) struct PlayerCreationStartup {
    pub(super) custom_spells: Arc<wow_data::PlayerCreateInfoCustomSpellStoreLikeCpp>,
    pub(super) cast_spells: Arc<wow_data::PlayerCreateInfoCastSpellStoreLikeCpp>,
    pub(super) stats: Arc<wow_data::PlayerStatsStore>,
    pub(super) create_info: Arc<wow_data::PlayerCreateInfoStoreLikeCpp>,
}

#[allow(clippy::too_many_arguments)]
pub(super) async fn load_player_creation_startup(
    data_dir: &str,
    locale: &str,
    player_creation_catalog_persistence:
        &dyn wow_persistence::PlayerCreationCatalogPersistencePortLikeCpp,
    player_base_stats_persistence: &dyn wow_persistence::PlayerBaseStatsPersistencePortLikeCpp,
    map_store: &wow_data::MapStore,
    chr_races_store: &wow_data::character_progression::ChrRacesStore,
    chr_classes_store: &wow_data::character_progression::ChrClassesStore,
    chr_model_store: &wow_data::character_progression::ChrModelStore,
    chr_race_x_chr_model_store: &wow_data::character_progression::ChrRaceXChrModelStore,
    gameobject_template_lifecycle_store: &wow_data::GameObjectTemplateLifecycleStoreLikeCpp,
    world_configs: &wow_config::WorldConfigSet,
) -> anyhow::Result<PlayerCreationStartup> {
    let player_create_taxi_path_store = wow_data::TaxiPathStore::load(&data_dir, &locale)
        .context("Failed to load TaxiPath.db2 for C++ playercreateinfo")?;
    let player_create_taxi_path_node_store = wow_data::TaxiPathNodeStore::load(&data_dir, &locale)
        .context("Failed to load TaxiPathNode.db2 for C++ playercreateinfo")?;
    let player_create_info_store = Arc::new(
        crate::player::creation_catalog::load_player_create_info_store_like_cpp(
            player_creation_catalog_persistence,
            map_store,
            chr_races_store,
            chr_classes_store,
            chr_model_store,
            chr_race_x_chr_model_store,
            gameobject_template_lifecycle_store,
            &player_create_taxi_path_store,
            &player_create_taxi_path_node_store,
        )
        .await
        .context("Failed to load C++ playercreateinfo base store")?,
    );
    let player_create_info_report = player_create_info_store.load_report_like_cpp().clone();
    info!(
        loaded = player_create_info_report.loaded,
        skipped_invalid_race = player_create_info_report.skipped_invalid_race,
        skipped_invalid_class = player_create_info_report.skipped_invalid_class,
        skipped_missing_gender_models = player_create_info_report.skipped_missing_gender_models,
        skipped_invalid_position = player_create_info_report.skipped_invalid_position,
        skipped_instanceable_map = player_create_info_report.skipped_instanceable_map,
        discarded_invalid_npe_map = player_create_info_report.discarded_invalid_npe_map,
        discarded_invalid_npe_transport = player_create_info_report.discarded_invalid_npe_transport,
        "Loaded C++ player create base definitions"
    );
    let valid_player_race_classes: Vec<_> = player_create_info_store
        .race_class_combinations_like_cpp()
        .collect();
    // C++ ObjectMgr::LoadPlayerInfo: class/level stats + race modifiers
    // only for `_playerInfo` race/class pairs, with create mana read from
    // gt/BaseMp.txt.
    let player_stats = Arc::new(
        crate::player::base_stats::load_player_base_stats_like_cpp(
            player_base_stats_persistence,
            &data_dir,
            world_config_u8(world_configs, "CONFIG_MAX_PLAYER_LEVEL", 80),
            &valid_player_race_classes,
        )
        .await
        .context("Failed to load C++ player class/race level stats")?,
    );
    info!(
        "Loaded {} C++ player race/class/level stat entries",
        player_stats.len()
    );
    let player_create_cast_spell_store = Arc::new(
        crate::player::creation_catalog::load_player_create_cast_spell_store_like_cpp(
            player_creation_catalog_persistence,
        )
        .await
        .context("Failed to load playercreateinfo_cast_spell")?,
    );
    let player_create_cast_spell_report = player_create_cast_spell_store
        .load_report_like_cpp()
        .clone();
    info!(
        loaded_assignments = player_create_cast_spell_report.loaded_assignments,
        skipped_invalid_race_mask = player_create_cast_spell_report.skipped_invalid_race_mask,
        skipped_invalid_class_mask = player_create_cast_spell_report.skipped_invalid_class_mask,
        skipped_invalid_create_mode = player_create_cast_spell_report.skipped_invalid_create_mode,
        "Loaded C++ player create cast spell assignments"
    );
    let player_create_custom_spell_store = Arc::new(
        crate::player::creation_catalog::load_player_create_custom_spell_store_like_cpp(
            player_creation_catalog_persistence,
        )
        .await
        .context("Failed to load playercreateinfo_spell_custom")?,
    );
    let player_create_custom_spell_report = player_create_custom_spell_store
        .load_report_like_cpp()
        .clone();
    info!(
        loaded_assignments = player_create_custom_spell_report.loaded_assignments,
        skipped_invalid_race_mask = player_create_custom_spell_report.skipped_invalid_race_mask,
        skipped_invalid_class_mask = player_create_custom_spell_report.skipped_invalid_class_mask,
        "Loaded C++ player create custom spell assignments"
    );

    Ok(PlayerCreationStartup {
        create_info: player_create_info_store,
        stats: player_stats,
        cast_spells: player_create_cast_spell_store,
        custom_spells: player_create_custom_spell_store,
    })
}
