//! Ordered object query startup composition.

use anyhow::Context;
use std::sync::Arc;
use tracing::info;
use crate::catalogs;

pub(super) struct ObjectQueryCatalogs {
    pub(super) object_mgr_catalogs: Arc<wow_world::session::ObjectMgrCatalogsLikeCpp>,
    pub(super) world_query_catalog_persistence: wow_database::world::query_catalog_adapter::MariaDbWorldQueryCatalogPersistenceAdapterLikeCpp,
    pub(super) _creature_quest_item_store: Arc<wow_data::CreatureQuestItemStoreLikeCpp>,
    pub(super) quest_item_catalog_persistence: wow_database::MariaDbQuestItemCatalogPersistenceAdapterLikeCpp,
}

pub(super) async fn load(
    world_db: &Arc<wow_database::WorldDatabase>,
    gameobject_template_lifecycle_store: &wow_data::GameObjectTemplateLifecycleStoreLikeCpp,
    creature_template_lifecycle_store: &wow_data::CreatureTemplateLifecycleStoreLikeCpp,
    item_stats_store: &wow_data::ItemStatsStore,
) -> anyhow::Result<ObjectQueryCatalogs> {
    let quest_item_catalog_persistence =
        wow_database::MariaDbQuestItemCatalogPersistenceAdapterLikeCpp::new(Arc::clone(world_db));
    let (gameobject_quest_item_store, creature_quest_item_store) =
        catalogs::quest_item::load_quest_item_catalogs_like_cpp(
            &quest_item_catalog_persistence,
            |entry| gameobject_template_lifecycle_store.get(entry).is_some(),
            |entry| creature_template_lifecycle_store.get(entry).is_some(),
            |item_id| item_stats_store.sparse_template(item_id).is_some(),
        )
        .await?;
    let gameobject_quest_item_store = Arc::new(gameobject_quest_item_store);
    let _creature_quest_item_store = Arc::new(creature_quest_item_store);

    let world_query_catalog_persistence =
        wow_database::world::query_catalog_adapter::MariaDbWorldQueryCatalogPersistenceAdapterLikeCpp::new(
            Arc::clone(world_db),
        );
    let (creature_query_catalog, gameobject_query_catalog, page_text_catalog) =
        crate::world::query_catalog::load_like_cpp(&world_query_catalog_persistence)
            .await
            .context("Failed to load immutable C++ ObjectMgr query catalogs")?;
    let object_mgr_catalogs = Arc::new(wow_world::session::ObjectMgrCatalogsLikeCpp {
        creature: Arc::new(creature_query_catalog),
        gameobject: Arc::new(gameobject_query_catalog),
        gameobject_quest_items: gameobject_quest_item_store,
        page_text: Arc::new(page_text_catalog),
    });
    info!(
        creatures = object_mgr_catalogs.creature.len(),
        gameobjects = object_mgr_catalogs.gameobject.len(),
        pages = object_mgr_catalogs.page_text.len(),
        "Loaded immutable C++ ObjectMgr query capability"
    );
    Ok(ObjectQueryCatalogs {
        quest_item_catalog_persistence,
        _creature_quest_item_store,
        world_query_catalog_persistence,
        object_mgr_catalogs,
    })
}
