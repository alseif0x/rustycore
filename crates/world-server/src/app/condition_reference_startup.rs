//! Immutable references used by condition and world-content validation.

use anyhow::Context;
use std::sync::Arc;
use tracing::info;

pub(super) struct ConditionReferences {
    pub(super) conversation_line_template_store: Arc<wow_data::WorldIdStore>,
    pub(super) conversation_line_store: Arc<wow_data::Db2IdStore>,
    pub(super) world_state_expression_store: Arc<wow_data::WorldStateExpressionStore>,
    pub(super) content_tuning_store: Arc<wow_data::progression_rewards::ContentTuningStore>,
    pub(super) adventure_map_poi_store: Arc<wow_data::AdventureMapPoiStore>,
    pub(super) player_condition_store: Arc<wow_data::PlayerConditionStore>,
    pub(super) scene_script_package_store: Arc<wow_data::Db2IdStore>,
    pub(super) scenario_step_store: Arc<wow_data::Db2IdStore>,
    pub(super) battle_pet_species_store: Arc<wow_data::Db2IdStore>,
    pub(super) char_titles_store: Arc<wow_data::Db2IdStore>,
    pub(super) battlemaster_list_typed_store: Arc<wow_data::BattlemasterListStore>,
    pub(super) battlemaster_list_store: Arc<wow_data::Db2IdStore>,
    pub(super) criteria_store: Arc<wow_data::Db2IdStore>,
    pub(super) achievement_store: Arc<wow_data::Db2IdStore>,
    pub(super) faction_store: Arc<wow_data::Db2IdStore>,
}

pub(super) async fn load(
    data_dir: &str,
    locale: &str,
    world_reference_catalog_persistence: &dyn wow_persistence::WorldReferenceCatalogPersistencePortLikeCpp,
) -> anyhow::Result<ConditionReferences> {
    let faction_store = Arc::new(
        wow_data::Db2IdStore::load(data_dir, locale, "Faction.db2")
            .context("Failed to load Faction.db2 — check DataDir and DBC.Locale config")?,
    );
    let achievement_store = Arc::new(
        wow_data::Db2IdStore::load(data_dir, locale, "Achievement.db2")
            .context("Failed to load Achievement.db2 — check DataDir and DBC.Locale config")?,
    );
    let criteria_store = Arc::new(
        wow_data::Db2IdStore::load(data_dir, locale, "Criteria.db2")
            .context("Failed to load Criteria.db2 — check DataDir and DBC.Locale config")?,
    );
    let battlemaster_list_store = Arc::new(
        wow_data::Db2IdStore::load(data_dir, locale, "BattlemasterList.db2")
            .context("Failed to load BattlemasterList.db2 — check DataDir and DBC.Locale config")?,
    );
    let battlemaster_list_typed_store = Arc::new(
        wow_data::BattlemasterListStore::load(data_dir, locale)
            .context("Failed to load typed BattlemasterList.db2 HolidayWorldState store")?,
    );
    let char_titles_store = Arc::new(
        wow_data::Db2IdStore::load(data_dir, locale, "CharTitles.db2")
            .context("Failed to load CharTitles.db2 — check DataDir and DBC.Locale config")?,
    );
    let battle_pet_species_store = Arc::new(
        wow_data::Db2IdStore::load(data_dir, locale, "BattlePetSpecies.db2")
            .context("Failed to load BattlePetSpecies.db2 — check DataDir and DBC.Locale config")?,
    );
    let scenario_step_store = Arc::new(
        wow_data::Db2IdStore::load(data_dir, locale, "ScenarioStep.db2")
            .context("Failed to load ScenarioStep.db2 — check DataDir and DBC.Locale config")?,
    );
    let scene_script_package_store = Arc::new(
        wow_data::Db2IdStore::load(data_dir, locale, "SceneScriptPackage.db2").context(
            "Failed to load SceneScriptPackage.db2 — check DataDir and DBC.Locale config",
        )?,
    );
    let player_condition_store = Arc::new(
        wow_data::PlayerConditionStore::load(data_dir, locale)
            .context("Failed to load PlayerCondition.db2 — check DataDir and DBC.Locale config")?,
    );
    let adventure_map_poi_store = Arc::new(
        wow_data::AdventureMapPoiStore::load(data_dir, locale)
            .context("Failed to load AdventureMapPOI.db2 — check DataDir and DBC.Locale config")?,
    );
    info!(
        "Loaded {} adventure map POIs from AdventureMapPOI.db2",
        adventure_map_poi_store.len()
    );
    let content_tuning_store = Arc::new(
        wow_data::progression_rewards::ContentTuningStore::load(data_dir, locale)
            .context("Failed to load ContentTuning.db2 — check DataDir and DBC.Locale config")?,
    );
    info!(
        "Loaded {} content tuning rows from ContentTuning.db2",
        content_tuning_store.len()
    );
    let world_state_expression_store = Arc::new(
        wow_data::WorldStateExpressionStore::load(data_dir, locale).context(
            "Failed to load WorldStateExpression.db2 — check DataDir and DBC.Locale config",
        )?,
    );
    let conversation_line_store = Arc::new(
        wow_data::Db2IdStore::load(data_dir, locale, "ConversationLine.db2")
            .context("Failed to load ConversationLine.db2 — check DataDir and DBC.Locale config")?,
    );
    let conversation_line_template_store = Arc::new(
        crate::world::reference_catalog::load_filtering_world_id_store_like_cpp(
            world_reference_catalog_persistence,
            wow_persistence::WorldObjectIdCatalogKindLikeCpp::ConversationLineTemplate,
            |line_id| conversation_line_store.contains(line_id),
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
        world_state_expression_store.len(),
        conversation_line_store.len()
    );
    info!(
        "Loaded condition validation conversation line template store: {} templates",
        conversation_line_template_store.len()
    );
    Ok(ConditionReferences {
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
        adventure_map_poi_store,
        content_tuning_store,
        world_state_expression_store,
        conversation_line_store,
        conversation_line_template_store,
    })
}

pub(super) struct SpawnReferences {
    pub(super) gameobject_template_store: Arc<wow_data::WorldIdStore>,
    pub(super) creature_template_store: Arc<wow_data::WorldIdStore>,
    pub(super) spawn_group_report: wow_data::SpawnGroupTemplateLoadReport,
    pub(super) spawn_group_store: wow_data::SpawnGroupTemplateStore,
}

pub(super) async fn load_spawn_references(
    world_auxiliary_catalog_persistence: &dyn wow_persistence::WorldAuxiliaryCatalogPersistencePortLikeCpp,
    world_reference_catalog_persistence: &dyn wow_persistence::WorldReferenceCatalogPersistencePortLikeCpp,
) -> anyhow::Result<SpawnReferences> {
    let (spawn_group_store, spawn_group_report) =
        crate::world::auxiliary_catalog::load_spawn_group_templates_like_cpp(
            world_auxiliary_catalog_persistence,
        )
        .await
        .context("Failed to load C++ spawn_group_template rows")?;
    info!(
        "Loaded {} spawn group templates ({} invalid flags, {} system/manual flag fixes, {} inserted defaults)",
        spawn_group_store.len(),
        spawn_group_report.invalid_flags.len(),
        spawn_group_report.system_manual_spawn_flags.len(),
        spawn_group_report.inserted_default_groups.len()
    );
    let creature_template_store = Arc::new(
        crate::world::reference_catalog::load_world_id_store_like_cpp(
            world_reference_catalog_persistence,
            wow_persistence::WorldObjectIdCatalogKindLikeCpp::CreatureTemplate,
        )
        .await
        .context("Failed to load creature_template ids for C++ ConditionMgr validation")?,
    );
    let gameobject_template_store = Arc::new(
        crate::world::reference_catalog::load_world_id_store_like_cpp(
            world_reference_catalog_persistence,
            wow_persistence::WorldObjectIdCatalogKindLikeCpp::GameObjectTemplate,
        )
        .await
        .context("Failed to load gameobject_template ids for C++ ConditionMgr validation")?,
    );
    info!(
        "Loaded condition validation world id stores: {} creature templates, {} gameobject templates",
        creature_template_store.len(),
        gameobject_template_store.len()
    );
    Ok(SpawnReferences {
        spawn_group_store,
        spawn_group_report,
        creature_template_store,
        gameobject_template_store,
    })
}

pub(super) struct SpawnIds {
    pub(super) gameobject_spawn_store: Arc<wow_data::WorldSpawnIdStore>,
    pub(super) creature_spawn_store: Arc<wow_data::WorldSpawnIdStore>,
}

pub(super) async fn load_spawn_ids(
    world_reference_catalog_persistence: &dyn wow_persistence::WorldReferenceCatalogPersistencePortLikeCpp,
) -> anyhow::Result<SpawnIds> {
    let creature_spawn_store = Arc::new(
        crate::world::reference_catalog::load_world_spawn_id_store_like_cpp(
            world_reference_catalog_persistence,
            wow_persistence::WorldSpawnCatalogKindLikeCpp::Creature,
        )
        .await
        .context("Failed to load creature spawn ids for C++ ConditionMgr validation")?,
    );
    let gameobject_spawn_store = Arc::new(
        crate::world::reference_catalog::load_world_spawn_id_store_like_cpp(
            world_reference_catalog_persistence,
            wow_persistence::WorldSpawnCatalogKindLikeCpp::GameObject,
        )
        .await
        .context("Failed to load gameobject spawn ids for C++ ConditionMgr validation")?,
    );
    info!(
        "Loaded condition validation spawn id stores: {} creature spawns, {} gameobject spawns",
        creature_spawn_store.len(),
        gameobject_spawn_store.len()
    );
    Ok(SpawnIds {
        creature_spawn_store,
        gameobject_spawn_store,
    })
}

pub(super) async fn load_trainer_ids(
    world_reference_catalog_persistence: &dyn wow_persistence::WorldReferenceCatalogPersistencePortLikeCpp,
) -> anyhow::Result<Arc<wow_data::WorldIdStore>> {
    let trainer_store = Arc::new(
        crate::world::reference_catalog::load_world_id_store_like_cpp(
            world_reference_catalog_persistence,
            wow_persistence::WorldObjectIdCatalogKindLikeCpp::Trainer,
        )
        .await
        .context("Failed to load trainer ids for C++ ConditionMgr validation")?,
    );
    info!(
        "Loaded condition validation trainer id store: {} trainers",
        trainer_store.len()
    );
    Ok(trainer_store)
}
