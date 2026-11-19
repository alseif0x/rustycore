//! Collection, condition and conversation catalogs produced during ordered startup.

use super::*;

pub(super) struct CollectionConditionCatalogs {
    pub(super) heirloom_store: Arc<wow_data::HeirloomStore>,
    pub(super) toy_store: Arc<wow_data::ToyStore>,
    pub(super) faction_store: Arc<wow_data::Db2IdStore>,
    pub(super) achievement_store: Arc<wow_data::Db2IdStore>,
    pub(super) criteria_store: Arc<wow_data::Db2IdStore>,
    pub(super) battlemaster_list_store: Arc<wow_data::Db2IdStore>,
    pub(super) battlemaster_list_typed_store: Arc<wow_data::BattlemasterListStore>,
    pub(super) char_titles_store: Arc<wow_data::Db2IdStore>,
    pub(super) battle_pet_species_store: Arc<wow_data::Db2IdStore>,
    pub(super) scenario_step_store: Arc<wow_data::Db2IdStore>,
    pub(super) scene_script_package_store: Arc<wow_data::Db2IdStore>,
    pub(super) player_condition_store: Arc<wow_data::PlayerConditionStore>,
    pub(super) condition_presence_catalog: Arc<wow_data::ConditionPresenceCatalog>,
    pub(super) adventure_map_poi_store: Arc<wow_data::AdventureMapPoiStore>,
    pub(super) content_tuning_store: Arc<wow_data::progression_rewards::ContentTuningStore>,
    pub(super) _world_state_expression_store: Arc<wow_data::WorldStateExpressionStore>,
    pub(super) _conversation_line_store: Arc<wow_data::Db2IdStore>,
    pub(super) conversation_line_template_store: Arc<wow_data::WorldIdStore>,
}

pub(super) async fn load_collection_condition_catalogs(
    data_dir: &str,
    locale: &str,
    hotfix_db: &Arc<HotfixDatabase>,
    db2_hotfix_removals: &wow_data::Db2HotfixRemovalStoreLikeCpp,
    world_reference_catalog_persistence: &wow_database::MariaDbWorldReferenceCatalogPersistenceAdapterLikeCpp,
) -> Result<CollectionConditionCatalogs> {
    let heirloom_store = Arc::new(
        wow_data::HeirloomStore::load(&data_dir, &locale)
            .context("Failed to load Heirloom.db2 — check DataDir and DBC.Locale config")?,
    );
    info!(
        "Loaded {} heirlooms from Heirloom.db2",
        heirloom_store.len()
    );
    let toy_store = Arc::new(
        wow_data::ToyStore::load(&data_dir, &locale)
            .context("Failed to load Toy.db2 — check DataDir and DBC.Locale config")?,
    );
    info!("Loaded {} toys from Toy.db2", toy_store.len());
    let faction_store = Arc::new(
        wow_data::Db2IdStore::load(&data_dir, &locale, "Faction.db2")
            .context("Failed to load Faction.db2 — check DataDir and DBC.Locale config")?,
    );
    let achievement_store = Arc::new(
        wow_data::Db2IdStore::load(&data_dir, &locale, "Achievement.db2")
            .context("Failed to load Achievement.db2 — check DataDir and DBC.Locale config")?,
    );
    let criteria_store = Arc::new(
        wow_data::Db2IdStore::load(&data_dir, &locale, "Criteria.db2")
            .context("Failed to load Criteria.db2 — check DataDir and DBC.Locale config")?,
    );
    let battlemaster_list_store = Arc::new(
        wow_data::Db2IdStore::load(&data_dir, &locale, "BattlemasterList.db2")
            .context("Failed to load BattlemasterList.db2 — check DataDir and DBC.Locale config")?,
    );
    let battlemaster_list_typed_store = Arc::new(
        wow_data::BattlemasterListStore::load(&data_dir, &locale)
            .context("Failed to load typed BattlemasterList.db2 HolidayWorldState store")?,
    );
    let char_titles_store = Arc::new(
        wow_data::Db2IdStore::load(&data_dir, &locale, "CharTitles.db2")
            .context("Failed to load CharTitles.db2 — check DataDir and DBC.Locale config")?,
    );
    let battle_pet_species_store = Arc::new(
        wow_data::Db2IdStore::load(&data_dir, &locale, "BattlePetSpecies.db2")
            .context("Failed to load BattlePetSpecies.db2 — check DataDir and DBC.Locale config")?,
    );
    let scenario_step_store = Arc::new(
        wow_data::Db2IdStore::load(&data_dir, &locale, "ScenarioStep.db2")
            .context("Failed to load ScenarioStep.db2 — check DataDir and DBC.Locale config")?,
    );
    let scene_script_package_store = Arc::new(
        wow_data::Db2IdStore::load(&data_dir, &locale, "SceneScriptPackage.db2").context(
            "Failed to load SceneScriptPackage.db2 — check DataDir and DBC.Locale config",
        )?,
    );
    let player_condition_store = Arc::new(
        wow_data::PlayerConditionStore::load(&data_dir, &locale)
            .context("Failed to load PlayerCondition.db2 — check DataDir and DBC.Locale config")?,
    );
    let condition_presence_persistence =
        wow_database::MariaDbConditionPresencePersistenceAdapter::new(Arc::clone(hotfix_db));
    let condition_presence_catalog =
        crate::hotfix::condition_presence::load_condition_presence_catalog(
            &data_dir,
            &locale,
            &condition_presence_persistence,
            db2_hotfix_removals,
        )
        .await
        .context("Failed to load effective PlayerCondition/UnitCondition presence")?;
    let adventure_map_poi_store = Arc::new(
        wow_data::AdventureMapPoiStore::load(&data_dir, &locale)
            .context("Failed to load AdventureMapPOI.db2 — check DataDir and DBC.Locale config")?,
    );
    info!(
        "Loaded {} adventure map POIs from AdventureMapPOI.db2",
        adventure_map_poi_store.len()
    );
    let content_tuning_store = Arc::new(
        wow_data::progression_rewards::ContentTuningStore::load(&data_dir, &locale)
            .context("Failed to load ContentTuning.db2 — check DataDir and DBC.Locale config")?,
    );
    info!(
        "Loaded {} content tuning rows from ContentTuning.db2",
        content_tuning_store.len()
    );
    let _world_state_expression_store = Arc::new(
        wow_data::WorldStateExpressionStore::load(&data_dir, &locale).context(
            "Failed to load WorldStateExpression.db2 — check DataDir and DBC.Locale config",
        )?,
    );
    let _conversation_line_store = Arc::new(
        wow_data::Db2IdStore::load(&data_dir, &locale, "ConversationLine.db2")
            .context("Failed to load ConversationLine.db2 — check DataDir and DBC.Locale config")?,
    );
    let conversation_line_template_store = Arc::new(
        crate::world::reference_catalog::load_filtering_world_id_store_like_cpp(
            world_reference_catalog_persistence,
            wow_persistence::WorldObjectIdCatalogKindLikeCpp::ConversationLineTemplate,
            |line_id| _conversation_line_store.contains(line_id),
        )
        .await
        .context("Failed to load conversation_line_template ids for C++ ConditionMgr validation")?,
    );
    info!(
        "Loaded condition validation DB2 id stores: {} factions, {} achievements, {} criteria, {} battlemaster lists, {} typed battlemaster holiday-world-state rows, {} titles, {} battle pet species, {} scenario steps, {} scene script packages, {} player conditions, {} world state expressions, {} conversation lines",
        faction_store.len(),
        achievement_store.len(),
        criteria_store.len(),
        battlemaster_list_store.len(),
        battlemaster_list_typed_store.len(),
        char_titles_store.len(),
        battle_pet_species_store.len(),
        scenario_step_store.len(),
        scene_script_package_store.len(),
        player_condition_store.len(),
        _world_state_expression_store.len(),
        _conversation_line_store.len()
    );
    info!(
        "Loaded condition validation conversation line template store: {} templates",
        conversation_line_template_store.len()
    );

    Ok(CollectionConditionCatalogs {
        heirloom_store,
        toy_store,
        faction_store,
        achievement_store,
        criteria_store,
        battlemaster_list_store,
        battlemaster_list_typed_store,
        char_titles_store,
        battle_pet_species_store,
        scenario_step_store,
        scene_script_package_store,
        player_condition_store,
        condition_presence_catalog,
        adventure_map_poi_store,
        content_tuning_store,
        _world_state_expression_store,
        _conversation_line_store,
        conversation_line_template_store,
    })
}
