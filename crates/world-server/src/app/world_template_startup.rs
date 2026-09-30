//! Ordered world template startup composition.

use anyhow::Context;
use std::sync::Arc;
use tracing::info;

pub(super) struct WorldTemplateStartup {
    pub(super) creature_runtime: super::creature_catalog_startup::CreatureRuntimeCatalogs,
    pub(super) _scene_template_store: Arc<wow_data::SceneTemplateStoreLikeCpp>,
    pub(super) scene_template_report: wow_data::SceneTemplateLoadReportLikeCpp,
    pub(super) script_name_interner: wow_data::ScriptNameInternerLikeCpp,
    pub(super) gameobject_override_lifecycle_store: Arc<wow_data::GameObjectOverrideLifecycleStoreLikeCpp>,
    pub(super) gameobject_template_lifecycle_store: Arc<wow_data::GameObjectTemplateLifecycleStoreLikeCpp>,
    pub(super) creature_template_sparring_store: Arc<wow_data::CreatureTemplateSparringStoreLikeCpp>,
    pub(super) creature_template_lifecycle_store: Arc<wow_data::CreatureTemplateLifecycleStoreLikeCpp>,
    pub(super) creature_template_classification_store: Arc<wow_data::CreatureTemplateClassificationStoreLikeCpp>,
    pub(super) spawn_references: super::condition_reference_startup::SpawnReferences,
}

pub(super) async fn load(
    data_dir: &str,
    locale: &str,
    hotfix_db: &Arc<wow_database::HotfixDatabase>,
    db2_hotfix_removals: &wow_data::Db2HotfixRemovalStoreLikeCpp,
    world_ports: &super::database_startup::WorldCatalogPorts,
    world_configs: &wow_config::WorldConfigSet,
) -> anyhow::Result<WorldTemplateStartup> {
    let spawn_references = super::condition_reference_startup::load_spawn_references(
        &world_ports.world_auxiliary_catalog_persistence, &world_ports.world_reference_catalog_persistence,
    ).await?;
    let super::creature_catalog_startup::CreatureTemplateCatalogs {
        creature_template_classification_store, mut creature_template_lifecycle_store,
        creature_template_sparring_store,
    } = super::creature_catalog_startup::load_creature_template_catalogs(
        &world_ports.world_object_catalog_persistence,
    )
    .await?;
    let gameobject_template_lifecycle_store = Arc::new(
        crate::world::object_catalog::load_gameobject_templates_like_cpp(
            &world_ports.world_object_catalog_persistence,
        )
            .await
            .context("Failed to load DB-backed gameobject_template lifecycle rows for C++ GameObject::LoadFromDB")?,
    );
    let gameobject_override_lifecycle_store = Arc::new(
        crate::world::object_catalog::load_gameobject_overrides_like_cpp(
            &world_ports.world_object_catalog_persistence,
        )
            .await
            .context("Failed to load DB-backed gameobject_overrides lifecycle rows for C++ GameObject::Create")?,
    );
    info!(
        "Loaded C++ GameObject lifecycle stores: {} template rows, {} spawn override rows",
        gameobject_template_lifecycle_store.len(),
        gameobject_override_lifecycle_store.len()
    );
    let mut script_name_interner = wow_data::build_template_script_name_interner_like_cpp(
        creature_template_lifecycle_store.as_ref(), gameobject_template_lifecycle_store.as_ref(),
    );
    let scene_template_outcome = crate::world::auxiliary_catalog::load_scene_templates_like_cpp(
        &world_ports.world_auxiliary_catalog_persistence, &mut script_name_interner,
    )
    .await
    .context("Failed to load C++ scene_template rows")?;
    let _scene_template_store = Arc::new(scene_template_outcome.store);
    info!(
        "Loaded {} C++ scene templates (C++ log-count bug would report {})",
        scene_template_outcome.report.rows_seen,
        scene_template_outcome.report.cpp_logged_count_bug_like_cpp
    );
    let creature_runtime = super::creature_catalog_startup::load_creature_runtime_catalogs(
        data_dir, locale, hotfix_db, db2_hotfix_removals, &world_ports.world_object_catalog_persistence,
        creature_template_classification_store.as_ref(), world_configs,
    )
    .await?;
    Ok(WorldTemplateStartup {
        spawn_references,
        creature_template_classification_store,
        creature_template_lifecycle_store,
        creature_template_sparring_store,
        gameobject_template_lifecycle_store,
        gameobject_override_lifecycle_store,
        script_name_interner,
        _scene_template_store,
        scene_template_report: scene_template_outcome.report,
        creature_runtime,
    })
}
