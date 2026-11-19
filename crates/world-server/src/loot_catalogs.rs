//! Loot and GameObject-for-quest catalogs produced during ordered startup.

use super::*;

pub(super) struct LootCatalogs {
    pub(super) loot_stores: Arc<LootStores>,
    pub(super) _gameobject_for_quest_store: Arc<wow_data::GameObjectForQuestStoreLikeCpp>,
}

pub(super) async fn load_loot_catalogs(
    world_db: &Arc<WorldDatabase>,
    item_store: &wow_data::ItemStore,
    gameobject_template_lifecycle_store: &wow_data::GameObjectTemplateLifecycleStoreLikeCpp,
) -> Result<LootCatalogs> {
    let loaded_loot_stores = load_loot_stores_like_cpp(world_db, item_store)
        .await
        .context("Failed to load C++ LootTemplates_* foundation stores")?;
    let loot_reference_report = check_loot_references_like_cpp(&loaded_loot_stores);
    log_loot_reference_report_like_cpp(&loot_reference_report);
    let loot_condition_ids = load_loot_condition_ids_like_cpp(world_db)
        .await
        .context("Failed to load C++ loot-template condition IDs")?;
    let mut loot_condition_report =
        check_loot_condition_links_like_cpp(&loaded_loot_stores, loot_condition_ids, |item_id| {
            item_store.get(item_id).is_some()
        });
    let loot_condition_reference_uses = load_loot_condition_reference_uses_like_cpp(world_db)
        .await
        .context("Failed to load C++ loot-template condition reference uses")?;
    let condition_reference_template_ids = load_condition_reference_template_ids_like_cpp(world_db)
        .await
        .context("Failed to load C++ condition reference template IDs")?;
    check_loot_condition_references_like_cpp(
        &mut loot_condition_report,
        loot_condition_reference_uses,
        condition_reference_template_ids,
    );
    log_loot_condition_link_report_like_cpp(&loot_condition_report);
    let loot_stores = Arc::new(loaded_loot_stores);
    let loaded_loot_templates: usize = loot_stores
        .values()
        .map(|store| store.templates().len())
        .sum();
    info!(
        "Loaded {} C++ loot-template stores with {} template IDs",
        loot_stores.len(),
        loaded_loot_templates
    );
    let _gameobject_for_quest_store = Arc::new(
        wow_data::GameObjectForQuestStoreLikeCpp::from_templates_like_cpp(
            gameobject_template_lifecycle_store,
            |loot_id| {
                loot_stores
                    .get(&LootStoreKind::Gameobject)
                    .is_some_and(|store| {
                        store.have_quest_loot_for_like_cpp(loot_id, loot_stores.as_ref())
                    })
            },
        ),
    );
    info!(
        "Loaded {} C++ GameObjects for quests",
        _gameobject_for_quest_store.len()
    );

    Ok(LootCatalogs {
        loot_stores,
        _gameobject_for_quest_store,
    })
}
