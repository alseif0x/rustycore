//! World lifecycle catalogs produced during ordered startup.

use super::*;

pub(super) struct WorldLifecycleCatalogs {
    pub(super) graveyard_store: wow_data::GraveyardStore,
    pub(super) gossip_store: wow_data::GossipStore,
    pub(super) gossip_catalog_adapter:
        Arc<wow_database::MariaDbGossipCatalogPersistenceAdapterLikeCpp>,
    pub(super) spawn_group_store: wow_data::SpawnGroupTemplateStore,
    pub(super) creature_template_store: Arc<wow_data::WorldIdStore>,
    pub(super) gameobject_template_store: Arc<wow_data::WorldIdStore>,
    pub(super) creature_template_classification_store:
        Arc<wow_data::CreatureTemplateClassificationStoreLikeCpp>,
    pub(super) creature_template_lifecycle_store:
        Arc<wow_data::CreatureTemplateLifecycleStoreLikeCpp>,
    pub(super) creature_template_sparring_store:
        Arc<wow_data::CreatureTemplateSparringStoreLikeCpp>,
    pub(super) gameobject_template_lifecycle_store:
        Arc<wow_data::GameObjectTemplateLifecycleStoreLikeCpp>,
    pub(super) gameobject_override_lifecycle_store:
        Arc<wow_data::GameObjectOverrideLifecycleStoreLikeCpp>,
    pub(super) script_name_interner: wow_data::ScriptNameInternerLikeCpp,
    pub(super) _scene_template_store: Arc<wow_data::SceneTemplateStoreLikeCpp>,
}

pub(super) async fn load_world_lifecycle_catalogs(
    world_auxiliary_catalog_persistence: &wow_database::MariaDbWorldAuxiliaryCatalogPersistenceAdapterLikeCpp,
    world_reference_catalog_persistence: &wow_database::MariaDbWorldReferenceCatalogPersistenceAdapterLikeCpp,
    world_object_catalog_persistence: &wow_database::MariaDbWorldObjectCatalogPersistenceAdapterLikeCpp,
    world_db: &Arc<WorldDatabase>,
    world_safe_loc_store: &wow_data::WorldSafeLocStore,
    area_table_store: &wow_data::AreaTableStore,
) -> Result<WorldLifecycleCatalogs> {
    let mut graveyard_store = wow_data::GraveyardStore::default();
    let graveyard_report = crate::world::auxiliary_catalog::load_graveyard_zones_like_cpp(
        world_auxiliary_catalog_persistence,
        &mut graveyard_store,
        |safe_loc_id| world_safe_loc_store.contains(safe_loc_id),
        |area_id| area_table_store.get(area_id).is_some(),
    )
    .await
    .context("Failed to load C++ graveyard_zone links")?;
    info!(
        "Loaded {} graveyard-zone links ({} missing safe locs, {} missing zones, {} duplicates)",
        graveyard_report.loaded,
        graveyard_report.missing_safe_locs.len(),
        graveyard_report.missing_zones.len(),
        graveyard_report.duplicates.len()
    );
    let gossip_catalog_adapter = Arc::new(
        wow_database::MariaDbGossipCatalogPersistenceAdapterLikeCpp::new(Arc::clone(world_db)),
    );
    let (gossip_store, gossip_load_report) =
        catalogs::gossip_startup::load_gossip_startup_catalog_like_cpp(
            gossip_catalog_adapter.as_ref(),
        )
        .await
        .context("Failed to load C++ gossip_menu/gossip_menu_option stores")?;
    info!(
        "Loaded {} gossip menu rows, {} gossip menu option rows, {} gossip_menu_option locale keys, and {} gossip_menu_addon rows",
        gossip_load_report.menu_rows,
        gossip_load_report.menu_item_rows,
        gossip_load_report.locale_entries,
        gossip_load_report.addon_rows
    );
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
    let creature_template_classification_store = Arc::new(
        crate::world::object_catalog::load_creature_classifications_like_cpp(
            world_object_catalog_persistence,
        )
            .await
            .context("Failed to load creature_template classifications for C++ creature difficulty damage rates")?,
    );
    let creature_template_lifecycle_store = Arc::new(
        crate::world::object_catalog::load_creature_templates_like_cpp(
            world_object_catalog_persistence,
        )
            .await
            .context("Failed to load DB-backed creature_template lifecycle rows for C++ Creature::LoadFromDB")?,
    );
    info!(
        "Loaded {} DB-backed creature_template lifecycle rows for loaded-grid Creature::LoadFromDB",
        creature_template_lifecycle_store.len()
    );
    let creature_template_sparring_store = Arc::new(
        crate::world::object_catalog::load_creature_sparring_like_cpp(
            world_object_catalog_persistence,
            creature_template_lifecycle_store.as_ref(),
        )
        .await
        .context("Failed to load creature_template_sparring rows for C++ Creature::LoadCreaturesSparringHealth")?,
    );
    info!(
        "Loaded {} creature template sparring rows",
        creature_template_sparring_store.len()
    );
    let gameobject_template_lifecycle_store = Arc::new(
        crate::world::object_catalog::load_gameobject_templates_like_cpp(
            world_object_catalog_persistence,
        )
            .await
            .context("Failed to load DB-backed gameobject_template lifecycle rows for C++ GameObject::LoadFromDB")?,
    );
    let gameobject_override_lifecycle_store = Arc::new(
        crate::world::object_catalog::load_gameobject_overrides_like_cpp(
            world_object_catalog_persistence,
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
        creature_template_lifecycle_store.as_ref(),
        gameobject_template_lifecycle_store.as_ref(),
    );
    let scene_template_outcome = crate::world::auxiliary_catalog::load_scene_templates_like_cpp(
        world_auxiliary_catalog_persistence,
        &mut script_name_interner,
    )
    .await
    .context("Failed to load C++ scene_template rows")?;
    let _scene_template_store = Arc::new(scene_template_outcome.store);
    info!(
        "Loaded {} C++ scene templates (C++ log-count bug would report {})",
        scene_template_outcome.report.rows_seen,
        scene_template_outcome.report.cpp_logged_count_bug_like_cpp
    );
    Ok(WorldLifecycleCatalogs {
        graveyard_store,
        gossip_store,
        gossip_catalog_adapter,
        spawn_group_store,
        creature_template_store,
        gameobject_template_store,
        creature_template_classification_store,
        creature_template_lifecycle_store,
        creature_template_sparring_store,
        gameobject_template_lifecycle_store,
        gameobject_override_lifecycle_store,
        script_name_interner,
        _scene_template_store,
    })
}
