//! Display, vehicle and spawn catalogs produced during ordered startup.

use super::*;

pub(super) struct DisplayVehicleCatalogs {
    pub(super) creature_model_data_store: Arc<wow_data::CreatureModelDataStore>,
    pub(super) creature_model_info_store: Arc<wow_data::CreatureModelInfoStoreLikeCpp>,
    pub(super) creature_display_info_extra_store: Arc<wow_data::CreatureDisplayInfoExtraStore>,
    pub(super) emotes_store: Arc<wow_data::EmotesStore>,
    pub(super) emotes_text_store: Arc<wow_data::EmotesTextStore>,
    pub(super) anim_kit_store: Arc<wow_data::AnimKitStore>,
    pub(super) movie_store: Arc<wow_data::MovieStore>,
    pub(super) cfg_categories_store: wow_data::CfgCategoriesStore,
    pub(super) gameobject_display_info_store: Arc<wow_data::GameObjectDisplayInfoStore>,
    pub(super) vehicle_store: Arc<wow_data::VehicleStore>,
    pub(super) vehicle_seat_store: Arc<wow_data::VehicleSeatStore>,
    pub(super) _vehicle_template_store: Arc<wow_data::VehicleTemplateStoreLikeCpp>,
    pub(super) vehicle_accessory_store: Arc<wow_data::VehicleAccessoryStoreLikeCpp>,
    pub(super) creature_spawn_store: Arc<wow_data::WorldSpawnIdStore>,
    pub(super) gameobject_spawn_store: Arc<wow_data::WorldSpawnIdStore>,
}

pub(super) async fn load_display_vehicle_catalogs(
    data_dir: &str,
    locale: &str,
    hotfix_db: &Arc<HotfixDatabase>,
    world_db: &Arc<WorldDatabase>,
    creature_display_hotfix_persistence: &wow_database::MariaDbCreatureDisplayHotfixPersistenceAdapterLikeCpp,
    creature_display_info_store: &wow_data::CreatureDisplayInfoStore,
    world_object_catalog_persistence: &wow_database::MariaDbWorldObjectCatalogPersistenceAdapterLikeCpp,
    world_reference_catalog_persistence: &wow_database::MariaDbWorldReferenceCatalogPersistenceAdapterLikeCpp,
) -> Result<DisplayVehicleCatalogs> {
    let creature_model_data_store = Arc::new(
        hotfix::creature_display::load_creature_model_data_store_like_cpp(
            &data_dir,
            &locale,
            creature_display_hotfix_persistence,
        )
        .await
        .context("Failed to load CreatureModelData.db2 / hotfix rows")?,
    );
    info!(
        "Loaded {} creature model data rows",
        creature_model_data_store.len()
    );
    let creature_model_info_store = Arc::new(
        crate::world::object_catalog::load_creature_model_info_like_cpp(
            world_object_catalog_persistence,
            creature_display_info_store,
            creature_model_data_store.as_ref(),
        )
        .await
        .context("Failed to load creature_model_info rows")?,
    );
    info!(
        "Loaded {} creature model info rows",
        creature_model_info_store.len()
    );
    let creature_display_info_extra_store = Arc::new(
        wow_data::CreatureDisplayInfoExtraStore::load(&data_dir, &locale)
            .context("Failed to load CreatureDisplayInfoExtra.db2")?,
    );
    info!(
        "Loaded {} creature display info extra rows",
        creature_display_info_extra_store.len()
    );
    let emotes_store = Arc::new(
        wow_data::EmotesStore::load(&data_dir, &locale).context("Failed to load Emotes.db2")?,
    );
    info!("Loaded {} emote rows", emotes_store.len());
    let emotes_text_store = Arc::new(
        wow_data::EmotesTextStore::load(&data_dir, &locale)
            .context("Failed to load EmotesText.db2")?,
    );
    info!("Loaded {} emote text rows", emotes_text_store.len());
    let anim_kit_store = Arc::new(
        wow_data::AnimKitStore::load(&data_dir, &locale).context("Failed to load AnimKit.db2")?,
    );
    info!("Loaded {} anim kit rows", anim_kit_store.len());
    let movie_store = Arc::new(
        wow_data::MovieStore::load(&data_dir, &locale).context("Failed to load Movie.db2")?,
    );
    info!("Loaded {} movie rows", movie_store.len());
    let cfg_categories_store = wow_data::CfgCategoriesStore::load(&data_dir, &locale)
        .context("Failed to load Cfg_Categories.db2")?;
    info!(
        "Loaded {} realm categories from Cfg_Categories.db2",
        cfg_categories_store.len()
    );
    let gameobject_display_info_store = Arc::new(
        wow_data::GameObjectDisplayInfoStore::load(&data_dir, &locale)
            .context("Failed to load GameObjectDisplayInfo.db2")?,
    );
    info!(
        "Loaded {} gameobject display info rows",
        gameobject_display_info_store.len()
    );
    let vehicle_hotfix_persistence =
        wow_database::MariaDbVehicleHotfixPersistenceAdapterLikeCpp::new(Arc::clone(hotfix_db));
    let vehicle_store = Arc::new(
        catalogs::vehicle::load_vehicle_store_like_cpp(
            &data_dir,
            &locale,
            &vehicle_hotfix_persistence,
        )
        .await
        .context("Failed to load Vehicle.db2 / hotfix rows")?,
    );
    info!("Loaded {} vehicle rows", vehicle_store.len());
    let vehicle_seat_store = Arc::new(
        catalogs::vehicle::load_vehicle_seat_store_like_cpp(
            &data_dir,
            &locale,
            &vehicle_hotfix_persistence,
        )
        .await
        .context("Failed to load VehicleSeat.db2 / hotfix rows")?,
    );
    info!("Loaded {} vehicle seat rows", vehicle_seat_store.len());
    let vehicle_world_persistence =
        wow_database::MariaDbVehicleWorldCatalogPersistenceAdapterLikeCpp::new(Arc::clone(
            world_db,
        ));
    let _vehicle_template_store = Arc::new(
        catalogs::vehicle::load_vehicle_template_store_like_cpp(&vehicle_world_persistence)
            .await
            .context("Failed to load C++ vehicle_template rows")?,
    );
    let vehicle_accessory_store = Arc::new(
        catalogs::vehicle::load_vehicle_accessory_store_like_cpp(&vehicle_world_persistence)
            .await
            .context("Failed to load C++ vehicle accessory rows")?,
    );
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
    Ok(DisplayVehicleCatalogs {
        creature_model_data_store,
        creature_model_info_store,
        creature_display_info_extra_store,
        emotes_store,
        emotes_text_store,
        anim_kit_store,
        movie_store,
        cfg_categories_store,
        gameobject_display_info_store,
        vehicle_store,
        vehicle_seat_store,
        _vehicle_template_store,
        vehicle_accessory_store,
        creature_spawn_store,
        gameobject_spawn_store,
    })
}
