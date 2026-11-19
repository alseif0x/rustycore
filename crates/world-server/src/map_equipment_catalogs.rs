//! Map difficulty, LFG and creature equipment catalogs produced during ordered startup.

use super::*;

pub(super) struct MapEquipmentCatalogs {
    pub(super) map_difficulty_store: Arc<wow_data::MapDifficultyStore>,
    pub(super) map_difficulty_x_condition_store: Arc<wow_data::MapDifficultyXConditionStore>,
    pub(super) lfg_dungeons_store: Arc<wow_data::LfgDungeonsStore>,
    pub(super) item_appearance_store: Arc<wow_data::ItemAppearanceStore>,
    pub(super) item_modified_appearance_store: Arc<wow_data::ItemModifiedAppearanceStore>,
    pub(super) item_stats_store: Arc<wow_data::ItemStatsStore>,
    pub(super) creature_equipment_store: Arc<wow_data::CreatureEquipmentStoreLikeCpp>,
}

pub(super) async fn load_map_equipment_catalogs(
    data_dir: &str,
    locale: &str,
    hotfix_db: &Arc<HotfixDatabase>,
    world_object_catalog_persistence: &wow_database::MariaDbWorldObjectCatalogPersistenceAdapterLikeCpp,
    creature_template_lifecycle_store: &wow_data::CreatureTemplateLifecycleStoreLikeCpp,
) -> Result<MapEquipmentCatalogs> {
    let map_difficulty_store = Arc::new(
        wow_data::MapDifficultyStore::load(&data_dir, &locale)
            .context("Failed to load MapDifficulty.db2 — check DataDir and DBC.Locale config")?,
    );
    info!(
        "Loaded {} map difficulties from MapDifficulty.db2",
        map_difficulty_store.len()
    );
    let map_difficulty_x_condition_store = Arc::new(
        wow_data::MapDifficultyXConditionStore::load(&data_dir, &locale).context(
            "Failed to load MapDifficultyXCondition.db2 — check DataDir and DBC.Locale config",
        )?,
    );
    info!(
        "Loaded {} map difficulty conditions from MapDifficultyXCondition.db2",
        map_difficulty_x_condition_store.len()
    );
    let lfg_dungeons_hotfix_persistence =
        wow_database::MariaDbLfgDungeonsHotfixPersistenceAdapterLikeCpp::new(Arc::clone(hotfix_db));
    let lfg_dungeons_store = Arc::new(
        hotfix::lfg_dungeons::load_lfg_dungeons_like_cpp(
            &data_dir,
            &locale,
            &lfg_dungeons_hotfix_persistence,
        )
        .await
        .context(
            "Failed to load LFGDungeons.db2 / hotfix rows — check DataDir and DBC.Locale config",
        )?,
    );
    info!(
        "Loaded {} LFG dungeons from LFGDungeons.db2 / hotfix rows",
        lfg_dungeons_store.len()
    );
    // Load item appearance/equipment dependencies before canonical SpawnStore metadata.
    // C++ `ObjectMgr::LoadCreatureData` validates `creature.equipment_id` through
    // `ObjectMgr::GetEquipmentInfo`, including `-1` random selection, while loading
    // CreatureData.
    let item_appearance_store = Arc::new(
        wow_data::ItemAppearanceStore::load(&data_dir, &locale)
            .context("Failed to load ItemAppearance.db2 — check DataDir and DBC.Locale config")?,
    );
    info!(
        "Loaded {} item appearances from ItemAppearance.db2",
        item_appearance_store.len()
    );
    let item_modified_appearance_store = Arc::new(
        wow_data::ItemModifiedAppearanceStore::load(&data_dir, &locale).context(
            "Failed to load ItemModifiedAppearance.db2 — check DataDir and DBC.Locale config",
        )?,
    );
    info!(
        "Loaded {} item modified appearances from ItemModifiedAppearance.db2",
        item_modified_appearance_store.len()
    );
    let item_stats_store = Arc::new(
        wow_data::ItemStatsStore::load(&data_dir, &locale)
            .context("Failed to load ItemSparse.db2 — check DataDir and DBC.Locale config")?,
    );
    info!(
        "Loaded {} items with stat modifiers from ItemSparse.db2",
        item_stats_store.len()
    );
    let creature_equipment_store = Arc::new(
        crate::world::object_catalog::load_creature_equipment_like_cpp(
            world_object_catalog_persistence,
            |entry| creature_template_lifecycle_store.get(entry).is_some(),
            |item_id| {
                item_stats_store
                    .sparse_template(item_id)
                    .map(|template| template.inventory_type as u8)
            },
            |item_id, appearance_mod_id| {
                item_modified_appearance_store
                    .get_for_item(item_id, appearance_mod_id)
                    .is_some()
            },
            |item_id| {
                item_modified_appearance_store
                    .get_default_for_item(item_id)
                    .and_then(|entry| u16::try_from(entry.item_appearance_modifier_id).ok())
            },
        )
        .await
        .context("Failed to load C++ creature equipment templates")?,
    );
    info!(
        "Loaded {} C++ creature equipment templates",
        creature_equipment_store.len()
    );

    Ok(MapEquipmentCatalogs {
        map_difficulty_store,
        map_difficulty_x_condition_store,
        lfg_dungeons_store,
        item_appearance_store,
        item_modified_appearance_store,
        item_stats_store,
        creature_equipment_store,
    })
}
