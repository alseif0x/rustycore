//! Ordered world-server startup and shutdown composition.
//!
//! Coordinates ordered bootstrap, listener publication, runtime launch and shutdown.

mod account_admission_startup;
mod area_trigger_template_startup;
mod collection_startup;
mod condition_reference_startup;
mod condition_startup;
mod creature_catalog_startup;
mod database_startup;
mod game_event_startup;
mod geography_startup;
mod group_startup;
mod guid_allocator_startup;
mod hotfix_delivery_startup;
mod inventory_catalogs;
mod item_auxiliary_catalogs;
mod jump_charge_startup;
mod listener_startup;
mod loot_startup;
mod network_configuration;
mod npc_service_catalogs;
mod object_lookup_startup;
mod object_query_startup;
mod player_catalog_startup;
mod player_choice_startup;
mod player_creation_startup;
mod presentation_startup;
mod process_startup;
mod progression_catalog_startup;
mod quest_admission_startup;
mod realm_startup;
mod runtime_launch;
mod runtime_supervision;
mod scaling_startup;
mod serve;
mod session_catalog_capabilities;
mod session_core_capabilities;
mod session_handler_policies;
mod session_persistence;
mod session_runtime_policy;
mod skill_catalogs;
mod spell_acquisition_startup;
mod spell_info_startup;
mod spell_pet_startup;
mod spell_world_startup;
mod stat_tables_startup;
mod vehicle_catalogs;
mod world_access_startup;
mod world_instance_startup;
mod world_object_startup;
mod world_startup;
mod world_state_startup;
mod world_template_startup;

use super::*;

/// Run the world server with explicit process arguments.
///
/// Boxing keeps the enormous startup future private to this crate and gives
/// embedders a stable, compact library boundary.
pub fn run(args: Vec<String>) -> Pin<Box<dyn Future<Output = Result<ExitCode>> + Send + 'static>> {
    run_with_modules(args, wow_module_api::ModuleRegistry::new())
}

/// Run the world server with a pre-composed trusted module registry.
///
/// The generated compositor crate (issue #229) calls this after invoking every
/// installed module's registrar in the operator's declared order. `run` is the
/// zero-module case and passes an empty registry, so the ordinary build is
/// unchanged and never observes a module.
pub fn run_with_modules(
    args: Vec<String>,
    modules: wow_module_api::ModuleRegistry,
) -> Pin<Box<dyn Future<Output = Result<ExitCode>> + Send + 'static>> {
    Box::pin(run_inner(args, Arc::new(modules)))
}

async fn run_inner(
    args: Vec<String>,
    modules: Arc<wow_module_api::ModuleRegistry>,
) -> Result<ExitCode> {
    let Some(process) = process_startup::initialize(args)? else {
        return Ok(ExitCode::SUCCESS);
    };
    // Keep the Login pool's owner before the other primary pools in drop order.
    let mut login_db: Option<LoginDatabase>;
    let (primary_login_db, char_db, world_db) = database_startup::open_primary_databases().await?;
    login_db = Some(primary_login_db);
    let world_ports = database_startup::compose_world_catalog_ports(&world_db)?;
    let player_base_stats_persistence =
        wow_database::MariaDbPlayerBaseStatsPersistenceAdapterLikeCpp::new(Arc::clone(&world_db));
    let player_creation_catalog_persistence =
        wow_database::MariaDbPlayerCreationCatalogPersistenceAdapterLikeCpp::new(Arc::clone(
            &world_db,
        ));
    let skill_world_rules_persistence =
        wow_database::MariaDbSkillWorldRulesPersistenceAdapterLikeCpp::new(Arc::clone(&world_db));

    let hotfix_db = database_startup::open_hotfix_database().await?;
    database_startup::validate_runtime_schemas(
        login_db
            .as_ref()
            .expect("Login database retained until account composition"),
        &char_db,
        world_db.as_ref(),
        &hotfix_db,
    )
    .await?;

    let hotfix_db = Arc::new(hotfix_db);
    let static_data_overlay_persistence =
        wow_database::MariaDbStaticDataOverlayPersistenceAdapterLikeCpp::new(
            Arc::clone(&hotfix_db),
            Arc::clone(&world_db),
        );
    let realm_availability = realm_startup::initialize_availability(
        login_db
            .as_ref()
            .expect("Login database retained until account composition"),
        &char_db,
        world_db.as_ref(),
    )
    .await?;
    let (id_generators, item_guid_allocator_advisory_lock) =
        guid_allocator_startup::initialize_guid_allocators(&char_db).await?;
    let mut item_guid_allocator_advisory_lock = Some(item_guid_allocator_advisory_lock);

    let char_db = Arc::new(char_db);

    // Load Item.db2 for inventory_type lookups (replaces item_type_cache table)
    let data_dir = wow_config::get_string_default("DataDir", "./Data");
    let locale_raw = wow_config::get_string_default("DBC.Locale", "0");
    let locale = locale_id_to_name(&locale_raw);
    let inventory = inventory_catalogs::load(&data_dir, &locale)?;

    let hotfix_delivery_metadata_persistence =
        wow_database::MariaDbHotfixDeliveryMetadataPersistenceAdapterLikeCpp::new(Arc::clone(
            &hotfix_db,
        ));
    let db2_hotfix_removals = crate::hotfix_delivery_metadata::load_db2_hotfix_removals_like_cpp(
        &hotfix_delivery_metadata_persistence,
    )
    .await
    .context("Failed to load effective DB2 hotfix removals")?;

    let specialization = player_catalog_startup::load_specialization(
        &data_dir,
        &locale,
        &hotfix_db,
        &db2_hotfix_removals,
    )
    .await?;
    let dungeon_encounter_store =
        world_instance_startup::load_dungeon_encounters(&data_dir, &locale)?;
    let geography = geography_startup::load_geography_base(
        &data_dir,
        &locale,
        &world_ports.world_reference_catalog_persistence,
        &static_data_overlay_persistence,
    )
    .await?;
    let fishing_base_skill_store = Arc::new(
        crate::skill_world_rules::load_fishing_base_skill_store_like_cpp(
            &skill_world_rules_persistence,
            &geography.area_table_store,
        )
        .await
        .context("Failed to load skill_fishing_base_level")?,
    );
    let geography_startup::PhaseAndGraveyardCatalogs {
        phase_hotfix_adapter,
        phase_store,
        phase_group_store,
        phase_world_adapter,
        mut phase_info_store,
        _phase_name_store,
        terrain_swap_store,
        graveyard_store: loaded_graveyard_store,
        graveyard_report: loaded_graveyard_report,
    } = geography_startup::load_phase_and_graveyard_catalogs(
        &data_dir,
        &locale,
        &hotfix_db,
        &world_db,
        geography.map_store.as_ref(),
        geography.area_table_store.as_ref(),
        geography.ui_map_x_map_art_store.as_ref(),
        geography.world_safe_loc_store.as_ref(),
        &world_ports.world_auxiliary_catalog_persistence,
    )
    .await?;
    let mut graveyard_store = Some(loaded_graveyard_store);
    let graveyard_report = loaded_graveyard_report;
    let gossip_catalog_adapter = Arc::new(
        wow_database::MariaDbGossipCatalogPersistenceAdapterLikeCpp::new(Arc::clone(&world_db)),
    );
    let (mut gossip_store, gossip_load_report) =
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
    world_startup::run_world_startup(
        &data_dir,
        &locale,
        &hotfix_db,
        &world_db,
        &char_db,
        &mut login_db,
        &hotfix_delivery_metadata_persistence,
        &db2_hotfix_removals,
        &world_ports,
        &player_base_stats_persistence,
        &player_creation_catalog_persistence,
        &skill_world_rules_persistence,
        &static_data_overlay_persistence,
        &inventory,
        &specialization,
        &dungeon_encounter_store,
        &geography,
        &fishing_base_skill_store,
        &phase_store,
        &phase_group_store,
        &mut phase_info_store,
        &terrain_swap_store,
        &mut graveyard_store,
        &gossip_catalog_adapter,
        &mut gossip_store,
        &process.world_configs,
        &process.world_runtime_state,
        &process.ip_location_store,
        &modules,
        &realm_availability,
        &id_generators,
        &mut item_guid_allocator_advisory_lock,
    )
    .await
}
