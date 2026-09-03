//! Ordered world-server startup and shutdown composition.
//!
//! This module intentionally remains above the normal physical-file target:
//! its body is the linear composition order inherited from C++ `Main.cpp` and
//! owns no gameplay algorithms. Splitting it further requires typed staged
//! bootstrap results so that resource identity, failure drops, task joins, and
//! shutdown order stay explicit; inventing one mega-context here would only
//! conceal those dependencies.

use super::*;
use wow_database::player_spell_acquisition_adapter::spell_acquisition_port;

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
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();
    wow_logging::install_panic_hook_like_cpp();

    let cli = WorldServerCliLikeCpp::parse_from(args);
    if cli.show_help {
        print!("{}", worldserver_cli_help_like_cpp());
        return Ok(ExitCode::SUCCESS);
    }
    if cli.show_version {
        println!("{}", worldserver_full_version_like_cpp());
        return Ok(ExitCode::SUCCESS);
    }

    let world_runtime_state = Arc::new(WorldRuntimeStateLikeCpp::new());

    info!("RustyCore World Server starting...");

    let config_report = load_world_config(&cli)?;
    log_startup_banner_like_cpp(&config_report);
    let world_configs = wow_config::load_world_config_values();
    create_pid_file_from_config_like_cpp()?;
    let ip_location_store = Arc::new(load_ip_location_from_config_like_cpp());
    // Connect to login database (needed for session key validation)
    let login_info = wow_config::get_database_info_default(
        "Login",
        DatabaseInfo::new("127.0.0.1", 3306, "trinity", "trinity", "auth"),
    );
    log_database_target_like_cpp("login", &login_info);

    let login_connection = wow_database::build_connection_string_with_ssl_like_cpp(
        &login_info.host,
        &login_info.port_or_socket,
        &login_info.username,
        &login_info.password,
        &login_info.database,
        login_info.ssl,
    );
    let login_db =
        LoginDatabase::open_with_pool_size(&login_connection, database_pool_size_like_cpp("Login"))
            .await
            .context("Failed to connect to login database")?;

    info!("Connected to login database");

    // Connect to character database
    let char_info = wow_config::get_database_info_default(
        "Character",
        DatabaseInfo::new("127.0.0.1", 3306, "trinity", "trinity", "characters"),
    );
    log_database_target_like_cpp("character", &char_info);

    let character_connection = wow_database::build_connection_string_with_ssl_like_cpp(
        &char_info.host,
        &char_info.port_or_socket,
        &char_info.username,
        &char_info.password,
        &char_info.database,
        char_info.ssl,
    );
    let char_db = CharacterDatabase::open_with_pool_size(
        &character_connection,
        database_pool_size_like_cpp("Character"),
    )
    .await
    .context("Failed to connect to character database")?;

    info!("Connected to character database");

    // Connect to world database
    let world_info = wow_config::get_database_info_default(
        "World",
        DatabaseInfo::new("127.0.0.1", 3306, "trinity", "trinity", "world"),
    );
    log_database_target_like_cpp("world", &world_info);

    let world_connection = wow_database::build_connection_string_with_ssl_like_cpp(
        &world_info.host,
        &world_info.port_or_socket,
        &world_info.username,
        &world_info.password,
        &world_info.database,
        world_info.ssl,
    );
    let world_db =
        WorldDatabase::open_with_pool_size(&world_connection, database_pool_size_like_cpp("World"))
            .await
            .context("Failed to connect to world database")?;

    info!("Connected to world database");
    let world_db = Arc::new(world_db);
    let world_reference_catalog_persistence =
        wow_database::MariaDbWorldReferenceCatalogPersistenceAdapterLikeCpp::new(Arc::clone(
            &world_db,
        ));
    let world_auxiliary_catalog_persistence =
        wow_database::MariaDbWorldAuxiliaryCatalogPersistenceAdapterLikeCpp::new(Arc::clone(
            &world_db,
        ));
    let world_object_catalog_persistence =
        wow_database::MariaDbWorldObjectCatalogPersistenceAdapterLikeCpp::new(Arc::clone(
            &world_db,
        ));
    let quest_catalog_persistence =
        wow_database::MariaDbQuestCatalogPersistenceAdapterLikeCpp::new(Arc::clone(&world_db));
    let gameplay_rule_catalog_persistence =
        wow_database::MariaDbGameplayRuleCatalogPersistenceAdapterLikeCpp::new(Arc::clone(
            &world_db,
        ));
    let condition_disable_catalog_persistence =
        wow_database::MariaDbConditionDisableCatalogPersistenceAdapterLikeCpp::new(Arc::clone(
            &world_db,
        ));
    let player_base_stats_persistence =
        wow_database::MariaDbPlayerBaseStatsPersistenceAdapterLikeCpp::new(Arc::clone(&world_db));
    let player_creation_catalog_persistence =
        wow_database::MariaDbPlayerCreationCatalogPersistenceAdapterLikeCpp::new(Arc::clone(
            &world_db,
        ));
    let skill_world_rules_persistence =
        wow_database::MariaDbSkillWorldRulesPersistenceAdapterLikeCpp::new(Arc::clone(&world_db));

    // Connect to hotfix database
    let hotfix_info = wow_config::get_database_info_default(
        "Hotfix",
        DatabaseInfo::new("127.0.0.1", 3306, "trinity", "trinity", "hotfixes"),
    );
    log_database_target_like_cpp("hotfix", &hotfix_info);

    let hotfix_connection = wow_database::build_connection_string_with_ssl_like_cpp(
        &hotfix_info.host,
        &hotfix_info.port_or_socket,
        &hotfix_info.username,
        &hotfix_info.password,
        &hotfix_info.database,
        hotfix_info.ssl,
    );
    let hotfix_db = HotfixDatabase::open_with_pool_size(
        &hotfix_connection,
        database_pool_size_like_cpp("Hotfix"),
    )
    .await
    .context("Failed to connect to hotfix database")?;

    info!("Connected to hotfix database");

    let migration_manifest = wow_database::migration::bundled_manifest()?;
    for (database, pool) in [
        (wow_database::migration::DatabaseKind::Auth, login_db.pool()),
        (
            wow_database::migration::DatabaseKind::Characters,
            char_db.pool(),
        ),
        (
            wow_database::migration::DatabaseKind::World,
            world_db.pool(),
        ),
        (
            wow_database::migration::DatabaseKind::Hotfixes,
            hotfix_db.pool(),
        ),
    ] {
        wow_database::migration::validate_runtime_schema(pool, &migration_manifest, database)
            .await?;
    }

    let hotfix_db = Arc::new(hotfix_db);
    let static_data_overlay_persistence =
        wow_database::MariaDbStaticDataOverlayPersistenceAdapterLikeCpp::new(
            Arc::clone(&hotfix_db),
            Arc::clone(&world_db),
        );
    let realm_id = realm_id_like_cpp()?;
    clear_online_accounts_like_cpp(&login_db, &char_db, realm_id).await?;
    verify_world_db_version_like_cpp(world_db.as_ref()).await?;
    set_realm_offline(&login_db, realm_id).await?;
    let realm_list = Arc::new(Mutex::new(RealmListSnapshotLikeCpp::default()));
    let realm_list_summary = update_realm_list_once_like_cpp(&login_db, &realm_list)
        .await
        .context("Failed to initialize RealmList from realmlist")?;
    info!(
        realms = realm_list_summary.realms,
        sub_regions = realm_list_summary.sub_regions,
        added = realm_list_summary.added,
        updated = realm_list_summary.updated,
        removed = realm_list_summary.removed,
        "Initialized RealmList from realmlist like C++"
    );
    let realm_list_update_handle = spawn_realm_list_update_loop_like_cpp(
        LoginDatabase::from_pool(login_db.pool().clone()),
        Arc::clone(&realm_list),
        realms_state_update_delay_secs_like_cpp(),
    );

    // Initialize GUID generator from MAX(guid) in characters table
    let max_guid = {
        let stmt = char_db.prepare(CharStatements::SEL_MAX_GUID);
        match char_db.query(&stmt).await {
            Ok(result) => {
                if result.is_empty() || result.is_null(0) {
                    1i64
                } else {
                    let max_val: u32 = result.try_read(0).unwrap_or(0);
                    (max_val as i64) + 1
                }
            }
            Err(_) => 1i64,
        }
    };

    let guid_generator = Arc::new(ObjectGuidGenerator::new(HighGuid::Player, max_guid));
    info!("GUID generator initialized, next counter: {max_guid}");

    // A process-local atomic generator is safe only while one world-server can
    // allocate for this character database. Hold a connection-scoped MySQL
    // advisory lock for the complete server lifetime, failing startup if a
    // rolling/duplicate process already owns that allocation domain.
    let mut item_guid_allocator_advisory_lock =
        ItemGuidAllocatorAdvisoryLockLikeCpp::acquire_like_cpp(char_db.pool())
            .await
            .context("failed to acquire the character DB item GUID allocator lock")?;

    // C++ `ObjectMgr::SetHighestGuids` initializes one process-wide item
    // generator from `MAX(item_instance.guid) + 1`.  Sharing the atomic Rust
    // mirror across every session prevents concurrent loot grants from
    // selecting the same database GUID.
    let next_item_guid = {
        let stmt = char_db.prepare(CharStatements::SEL_MAX_ITEM_GUID);
        match char_db.query(&stmt).await {
            Ok(result) => {
                if result.is_empty() || result.is_null(0) {
                    next_item_guid_allocator_start_like_cpp(None)?
                } else {
                    let max_val: u64 = result
                        .try_read(0)
                        .context("failed to decode MAX(item_instance.guid)")?;
                    next_item_guid_allocator_start_like_cpp(Some(max_val))?
                }
            }
            Err(error) => {
                return Err(error)
                    .context("failed to initialize item GUID allocator from item_instance");
            }
        }
    };
    let next_item_guid_u64 = u64::try_from(next_item_guid)
        .context("item GUID allocator start must be a positive database counter")?;
    char_db
        .commit_transaction(item_guid_reference_cleanup_transaction_like_cpp(
            &char_db,
            next_item_guid_u64,
        ))
        .await
        .context("failed to clean dangling item GUID references before allocator publication")?;
    let item_guid_generator = Arc::new(ObjectGuidGenerator::new(HighGuid::Item, next_item_guid));
    info!("Item GUID generator initialized, next counter: {next_item_guid}");

    // C++ `ObjectMgr::SetHighestGuids` owns one raw uint64 namespace for both
    // equipment sets and transmog outfits. It must be initialized only after
    // the process has exclusive ownership of this character database's GUID
    // allocation domain (the advisory lock above).
    let next_equipment_set_guid = {
        let stmt = char_db.prepare(CharStatements::SEL_MAX_EQUIPMENT_SET_GUID);
        match char_db.query(&stmt).await {
            Ok(result) => {
                if result.is_empty() || result.is_null(0) {
                    next_equipment_set_guid_allocator_start_like_cpp(None)?
                } else {
                    let max_val: u64 = result
                        .try_read(0)
                        .context("failed to decode the equipment/transmog set GUID maximum")?;
                    next_equipment_set_guid_allocator_start_like_cpp(Some(max_val))?
                }
            }
            Err(error) => {
                return Err(error).context(
                    "failed to initialize the equipment-set GUID allocator from character_equipmentsets/character_transmog_outfits",
                );
            }
        }
    };
    let equipment_set_guid_generator = Arc::new(EquipmentSetGuidGeneratorLikeCpp::new(
        next_equipment_set_guid,
    ));
    info!("Equipment-set GUID generator initialized, next counter: {next_equipment_set_guid}");

    // C++ `ObjectMgr::SetHighestGuids` initializes a second raw uint64
    // namespace for `character_void_storage.itemId`. Keep it under the same
    // process/CharacterDB allocator ownership lock as item and equipment IDs.
    let next_void_storage_item_id = {
        let stmt = char_db.prepare(CharStatements::SEL_MAX_VOID_STORAGE_ITEM_ID);
        match char_db.query(&stmt).await {
            Ok(result) => {
                if result.is_empty() || result.is_null(0) {
                    next_void_storage_item_id_allocator_start_like_cpp(None)?
                } else {
                    let max_val: u64 = result
                        .try_read(0)
                        .context("failed to decode the void-storage item ID maximum")?;
                    next_void_storage_item_id_allocator_start_like_cpp(Some(max_val))?
                }
            }
            Err(error) => {
                return Err(error).context(
                    "failed to initialize the void-storage item ID allocator from character_void_storage",
                );
            }
        }
    };
    let void_storage_item_id_generator = Arc::new(VoidStorageItemIdGeneratorLikeCpp::new(
        next_void_storage_item_id,
    ));
    info!("Void-storage item ID generator initialized, next counter: {next_void_storage_item_id}");

    let char_db = Arc::new(char_db);

    // Load Item.db2 for inventory_type lookups (replaces item_type_cache table)
    let data_dir = wow_config::get_string_default("DataDir", "./Data");
    let locale_raw = wow_config::get_string_default("DBC.Locale", "0");
    let locale = locale_id_to_name(&locale_raw);
    let currency_types_store = Arc::new(
        wow_data::CurrencyTypesStore::load(&data_dir, &locale)
            .context("Failed to load CurrencyTypes.db2 — check DataDir and DBC.Locale config")?,
    );
    info!(
        "Loaded {} currencies from CurrencyTypes.db2",
        currency_types_store.len()
    );

    let import_price_stores = Arc::new(
        wow_data::ImportPriceStores::load(&data_dir, &locale)
            .context("Failed to load ImportPrice*.db2 — check DataDir and DBC.Locale config")?,
    );
    info!("Loaded ImportPrice*.db2 stores");

    let bank_bag_slot_prices_store = Arc::new(
        wow_data::BankBagSlotPricesStore::load(&data_dir, &locale).context(
            "Failed to load BankBagSlotPrices.db2 — check DataDir and DBC.Locale config",
        )?,
    );
    info!(
        "Loaded {} bank bag slot price rows from BankBagSlotPrices.db2",
        bank_bag_slot_prices_store.len()
    );

    let item_class_store = Arc::new(
        wow_data::ItemClassStore::load(&data_dir, &locale)
            .context("Failed to load ItemClass.db2 — check DataDir and DBC.Locale config")?,
    );
    info!(
        "Loaded {} item classes from ItemClass.db2",
        item_class_store.len()
    );

    let item_currency_cost_store = Arc::new(
        wow_data::ItemCurrencyCostStore::load(&data_dir, &locale)
            .context("Failed to load ItemCurrencyCost.db2 — check DataDir and DBC.Locale config")?,
    );
    info!(
        "Loaded {} item currency costs from ItemCurrencyCost.db2",
        item_currency_cost_store.len()
    );

    let item_extended_cost_store = Arc::new(
        wow_data::ItemExtendedCostStore::load(&data_dir, &locale)
            .context("Failed to load ItemExtendedCost.db2 — check DataDir and DBC.Locale config")?,
    );
    info!(
        "Loaded {} item extended costs from ItemExtendedCost.db2",
        item_extended_cost_store.len()
    );

    let item_store = Arc::new(
        wow_data::ItemStore::load(&data_dir, &locale)
            .context("Failed to load Item.db2 — check DataDir and DBC.Locale config")?,
    );
    info!("Loaded {} items from Item.db2", item_store.len());
    let item_child_equipment_store = Arc::new(
        wow_data::ItemChildEquipmentStore::load(&data_dir, &locale).context(
            "Failed to load ItemChildEquipment.db2 — check DataDir and DBC.Locale config",
        )?,
    );
    info!(
        "Loaded {} item child-equipment rows from ItemChildEquipment.db2",
        item_child_equipment_store.len()
    );

    let item_price_base_store = Arc::new(
        wow_data::ItemPriceBaseStore::load(&data_dir, &locale)
            .context("Failed to load ItemPriceBase.db2 — check DataDir and DBC.Locale config")?,
    );
    info!(
        "Loaded {} item price base rows from ItemPriceBase.db2",
        item_price_base_store.len()
    );

    let item_limit_category_store = Arc::new(
        wow_data::ItemLimitCategoryStore::load(&data_dir, &locale).context(
            "Failed to load ItemLimitCategory.db2 — check DataDir and DBC.Locale config",
        )?,
    );
    info!(
        "Loaded {} item limit categories from ItemLimitCategory.db2",
        item_limit_category_store.len()
    );

    let item_limit_category_condition_store = Arc::new(
        wow_data::ItemLimitCategoryConditionStore::load(&data_dir, &locale).context(
            "Failed to load ItemLimitCategoryCondition.db2 — check DataDir and DBC.Locale config",
        )?,
    );
    info!(
        "Loaded {} item limit category conditions from ItemLimitCategoryCondition.db2",
        item_limit_category_condition_store.len()
    );

    let item_bonus_db2_store = Arc::new(
        wow_data::ItemBonusDb2Store::load(&data_dir, &locale)
            .context("Failed to load ItemBonus.db2 — check DataDir and DBC.Locale config")?,
    );
    info!(
        "Loaded {} item bonus rows from ItemBonus.db2",
        item_bonus_db2_store.len()
    );
    let pvp_item_store = Arc::new(
        wow_data::PvpItemStore::load(&data_dir, &locale)
            .context("Failed to load PVPItem.db2 — check DataDir and DBC.Locale config")?,
    );
    info!(
        "Loaded {} PvP item bonus rows from PVPItem.db2",
        pvp_item_store.len()
    );
    let item_set_store = Arc::new(
        wow_data::ItemSetStore::load(&data_dir, &locale)
            .context("Failed to load ItemSet.db2 — check DataDir and DBC.Locale config")?,
    );
    info!(
        "Loaded {} item set rows from ItemSet.db2",
        item_set_store.len()
    );
    let item_set_spell_store = Arc::new(
        wow_data::ItemSetSpellStore::load(&data_dir, &locale)
            .context("Failed to load ItemSetSpell.db2 — check DataDir and DBC.Locale config")?,
    );
    info!(
        "Loaded {} item set spell rows from ItemSetSpell.db2",
        item_set_spell_store.len()
    );

    let hotfix_delivery_metadata_persistence =
        wow_database::MariaDbHotfixDeliveryMetadataPersistenceAdapterLikeCpp::new(Arc::clone(
            &hotfix_db,
        ));
    let db2_hotfix_removals = crate::hotfix_delivery_metadata::load_db2_hotfix_removals_like_cpp(
        &hotfix_delivery_metadata_persistence,
    )
    .await
    .context("Failed to load effective DB2 hotfix removals")?;

    // Load effective ChrSpecialization authority for C++ specialization validation.
    let chr_specialization_hotfix_persistence =
        wow_database::MariaDbChrSpecializationHotfixPersistenceAdapterLikeCpp::new(Arc::clone(
            &hotfix_db,
        ));
    let chr_specialization_store = Arc::new(
        crate::chr_specialization_hotfix::load_chr_specialization_store_like_cpp(
            &data_dir,
            &locale,
            &chr_specialization_hotfix_persistence,
            &db2_hotfix_removals,
        )
        .await
        .context(
            "Failed to load effective ChrSpecialization store — check DataDir and DBC.Locale config",
        )?,
    );
    info!(
        "Loaded {} effective chr specializations from ChrSpecialization.db2 and SQL overlays",
        chr_specialization_store.len()
    );

    // Load DungeonEncounter.db2 for C++ instance encounter lock/loot metadata.
    let dungeon_encounter_store = Arc::new(
        wow_data::DungeonEncounterStore::load(&data_dir, &locale)
            .context("Failed to load DungeonEncounter.db2 — check DataDir and DBC.Locale config")?,
    );
    info!(
        "Loaded {} dungeon encounters from DungeonEncounter.db2",
        dungeon_encounter_store.len()
    );

    // Load Map.db2 + MapDifficulty.db2 for C++ InstanceLockMgr MapDb2Entries resolution.
    let map_store = Arc::new(
        wow_data::MapStore::load(&data_dir, &locale)
            .context("Failed to load Map.db2 — check DataDir and DBC.Locale config")?,
    );
    info!("Loaded {} maps from Map.db2", map_store.len());
    let (world_safe_loc_store, world_safe_loc_report) =
        crate::world_reference_catalog::load_world_safe_locs_like_cpp(
            &world_reference_catalog_persistence,
            &map_store,
        )
        .await
        .context("Failed to load C++ world_safe_locs")?;
    info!(
        "Loaded {} world safe locs ({} missing maps, {} invalid positions)",
        world_safe_loc_store.len(),
        world_safe_loc_report.missing_maps.len(),
        world_safe_loc_report.invalid_positions.len()
    );
    let world_safe_loc_store = Arc::new(world_safe_loc_store);
    let ui_map_x_map_art_store = Arc::new(
        crate::static_data_overlay::load_ui_map_x_map_art_store_like_cpp(
            &data_dir,
            &locale,
            &static_data_overlay_persistence,
        )
        .await
        .context("Failed to load UiMapXMapArt.db2 / hotfix rows")?,
    );
    let area_table_store = Arc::new(
        crate::static_data_overlay::load_area_table_store_like_cpp(
            &data_dir,
            &locale,
            &static_data_overlay_persistence,
        )
        .await
        .context("Failed to load AreaTable.db2 / hotfix rows")?,
    );
    let area_trigger_db2_store = Arc::new(
        wow_data::AreaTriggerDb2Store::load(&data_dir, &locale)
            .context("Failed to load AreaTrigger.db2 — check DataDir and DBC.Locale config")?,
    );
    info!(
        "Loaded {} area trigger DB2 rows from AreaTrigger.db2",
        area_trigger_db2_store.len()
    );
    let fishing_base_skill_store = Arc::new(
        crate::skill_world_rules::load_fishing_base_skill_store_like_cpp(
            &skill_world_rules_persistence,
            &area_table_store,
        )
        .await
        .context("Failed to load skill_fishing_base_level")?,
    );
    let phase_hotfix_adapter =
        wow_database::MariaDbPhaseHotfixPersistenceAdapterLikeCpp::new(Arc::clone(&hotfix_db));
    let (phase_store, phase_group_store) = crate::phase_hotfix_catalog::load_phase_stores_like_cpp(
        &data_dir,
        &locale,
        &phase_hotfix_adapter,
    )
    .await
    .context("Failed to load Phase/PhaseXPhaseGroup DB2 and hotfix rows")?;
    let phase_store = Arc::new(phase_store);
    let phase_group_store = Arc::new(phase_group_store);
    info!(
        "Loaded {} phases and {} phase-group rows",
        phase_store.len(),
        phase_group_store.len()
    );
    let phase_world_adapter =
        wow_database::MariaDbPhaseWorldCatalogPersistenceAdapterLikeCpp::new(Arc::clone(&world_db));
    let (mut phase_info_store, phase_name_store, terrain_swap_store) =
        crate::phase_world_catalog::load_phase_world_catalogs_like_cpp(
            &phase_world_adapter,
            &area_table_store,
            &phase_store,
            &map_store,
            |phase_id| ui_map_x_map_art_store.is_ui_map_phase(phase_id),
        )
        .await?;
    let _phase_name_store = Arc::new(phase_name_store);
    let terrain_swap_store = Arc::new(terrain_swap_store);
    let mut graveyard_store = wow_data::GraveyardStore::default();
    let graveyard_report = crate::world_auxiliary_catalog::load_graveyard_zones_like_cpp(
        &world_auxiliary_catalog_persistence,
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
        wow_database::MariaDbGossipCatalogPersistenceAdapterLikeCpp::new(Arc::clone(&world_db)),
    );
    let (mut gossip_store, gossip_load_report) =
        crate::gossip_startup_catalog::load_gossip_startup_catalog_like_cpp(
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
        crate::world_auxiliary_catalog::load_spawn_group_templates_like_cpp(
            &world_auxiliary_catalog_persistence,
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
        crate::world_reference_catalog::load_world_id_store_like_cpp(
            &world_reference_catalog_persistence,
            wow_persistence::WorldObjectIdCatalogKindLikeCpp::CreatureTemplate,
        )
        .await
        .context("Failed to load creature_template ids for C++ ConditionMgr validation")?,
    );
    let gameobject_template_store = Arc::new(
        crate::world_reference_catalog::load_world_id_store_like_cpp(
            &world_reference_catalog_persistence,
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
        crate::world_object_catalog::load_creature_classifications_like_cpp(
            &world_object_catalog_persistence,
        )
            .await
            .context("Failed to load creature_template classifications for C++ creature difficulty damage rates")?,
    );
    let mut creature_template_lifecycle_store = Arc::new(
        crate::world_object_catalog::load_creature_templates_like_cpp(
            &world_object_catalog_persistence,
        )
            .await
            .context("Failed to load DB-backed creature_template lifecycle rows for C++ Creature::LoadFromDB")?,
    );
    info!(
        "Loaded {} DB-backed creature_template lifecycle rows for loaded-grid Creature::LoadFromDB",
        creature_template_lifecycle_store.len()
    );
    let creature_template_sparring_store = Arc::new(
        crate::world_object_catalog::load_creature_sparring_like_cpp(
            &world_object_catalog_persistence,
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
        crate::world_object_catalog::load_gameobject_templates_like_cpp(
            &world_object_catalog_persistence,
        )
            .await
            .context("Failed to load DB-backed gameobject_template lifecycle rows for C++ GameObject::LoadFromDB")?,
    );
    let gameobject_override_lifecycle_store = Arc::new(
        crate::world_object_catalog::load_gameobject_overrides_like_cpp(
            &world_object_catalog_persistence,
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
    let scene_template_outcome = crate::world_auxiliary_catalog::load_scene_templates_like_cpp(
        &world_auxiliary_catalog_persistence,
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
    let creature_damage_rates = wow_data::CreatureClassificationDamageRatesLikeCpp {
        normal: world_config_f32(&world_configs, "Rate.Creature.Damage.Normal", 1.0),
        elite: world_config_f32(&world_configs, "Rate.Creature.Damage.Elite", 1.0),
        rare_elite: world_config_f32(&world_configs, "Rate.Creature.Damage.RareElite", 1.0),
        obsolete: world_config_f32(&world_configs, "Rate.Creature.Damage.Obsolete", 1.0),
        rare: world_config_f32(&world_configs, "Rate.Creature.Damage.Rare", 1.0),
        trivial: world_config_f32(&world_configs, "Rate.Creature.Damage.Trivial", 1.0),
        minus_mob: world_config_f32(&world_configs, "Rate.Creature.Damage.MinusMob", 1.0),
    };
    let creature_health_rates = wow_data::CreatureClassificationHealthRatesLikeCpp {
        normal: world_config_f32(&world_configs, "Rate.Creature.Health.Normal", 1.0),
        elite: world_config_f32(&world_configs, "Rate.Creature.Health.Elite", 1.0),
        rare_elite: world_config_f32(&world_configs, "Rate.Creature.Health.RareElite", 1.0),
        obsolete: world_config_f32(&world_configs, "Rate.Creature.Health.Obsolete", 1.0),
        rare: world_config_f32(&world_configs, "Rate.Creature.Health.Rare", 1.0),
        trivial: world_config_f32(&world_configs, "Rate.Creature.Health.Trivial", 1.0),
        minus_mob: world_config_f32(&world_configs, "Rate.Creature.Health.MinusMob", 1.0),
    };
    let difficulty_hotfix_persistence =
        wow_database::MariaDbDifficultyHotfixPersistenceAdapterLikeCpp::new(Arc::clone(&hotfix_db));
    let difficulty_store = Arc::new(
        crate::difficulty_hotfix::load_difficulty_store_like_cpp(
            &data_dir,
            &locale,
            &difficulty_hotfix_persistence,
            &db2_hotfix_removals,
        )
        .await
        .context(
            "Failed to load effective Difficulty store — check DataDir and DBC.Locale config",
        )?,
    );
    info!(
        "Loaded {} effective difficulties from Difficulty.db2 and SQL overlays",
        difficulty_store.len()
    );
    let creature_difficulty_store = Arc::new(
        crate::world_object_catalog::load_creature_difficulties_like_cpp(
            &world_object_catalog_persistence,
            &difficulty_store,
            |entry| {
                // C++ missing-template rows are skipped before insertion. This data-wiring
                // slice does not invent full templates; if the minimal classification row is
                // absent, fall back to classification 1 (elite), matching
                // Creature::GetDamageMod's default switch rate.
                let classification = creature_template_classification_store
                    .classification_for_entry(entry)
                    .unwrap_or(1);
                creature_damage_rates.modifier_for_classification_like_cpp(classification)
            },
        )
        .await
        .context(
            "Failed to load creature_template_difficulty rows with C++ classification damage rates",
        )?,
    );
    let creature_base_stats_store = Arc::new(
        crate::world_object_catalog::load_creature_base_stats_like_cpp(
            &world_object_catalog_persistence,
        )
        .await
        .context("Failed to load creature_classlevelstats rows")?,
    );
    info!(
        "Loaded C++ creature runtime data stores: {} template classifications, {} difficulty rows, {} base stat rows",
        creature_template_classification_store.len(),
        creature_difficulty_store.len(),
        creature_base_stats_store.len()
    );
    let creature_template_mount_store = Arc::new(
        crate::world_object_catalog::load_creature_mounts_like_cpp(
            &world_object_catalog_persistence,
        )
        .await
        .context("Failed to load creature_template mount fallback rows")?,
    );
    info!(
        "Loaded {} creature template mount fallback rows",
        creature_template_mount_store.len()
    );
    let creature_display_hotfix_persistence =
        wow_database::MariaDbCreatureDisplayHotfixPersistenceAdapterLikeCpp::new(Arc::clone(
            &hotfix_db,
        ));
    let creature_display_info_store = Arc::new(
        crate::creature_display_hotfix::load_creature_display_info_store_like_cpp(
            &data_dir,
            &locale,
            &creature_display_hotfix_persistence,
        )
        .await
        .context("Failed to load CreatureDisplayInfo.db2 / hotfix rows")?,
    );
    info!(
        "Loaded {} creature display info rows",
        creature_display_info_store.len()
    );
    let creature_model_data_store = Arc::new(
        crate::creature_display_hotfix::load_creature_model_data_store_like_cpp(
            &data_dir,
            &locale,
            &creature_display_hotfix_persistence,
        )
        .await
        .context("Failed to load CreatureModelData.db2 / hotfix rows")?,
    );
    info!(
        "Loaded {} creature model data rows",
        creature_model_data_store.len()
    );
    let creature_model_info_store = Arc::new(
        crate::world_object_catalog::load_creature_model_info_like_cpp(
            &world_object_catalog_persistence,
            creature_display_info_store.as_ref(),
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
        wow_database::MariaDbVehicleHotfixPersistenceAdapterLikeCpp::new(Arc::clone(&hotfix_db));
    let vehicle_store = Arc::new(
        crate::vehicle_catalog::load_vehicle_store_like_cpp(
            &data_dir,
            &locale,
            &vehicle_hotfix_persistence,
        )
        .await
        .context("Failed to load Vehicle.db2 / hotfix rows")?,
    );
    info!("Loaded {} vehicle rows", vehicle_store.len());
    let vehicle_seat_store = Arc::new(
        crate::vehicle_catalog::load_vehicle_seat_store_like_cpp(
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
            &world_db,
        ));
    let _vehicle_template_store = Arc::new(
        crate::vehicle_catalog::load_vehicle_template_store_like_cpp(&vehicle_world_persistence)
            .await
            .context("Failed to load C++ vehicle_template rows")?,
    );
    let vehicle_accessory_store = Arc::new(
        crate::vehicle_catalog::load_vehicle_accessory_store_like_cpp(&vehicle_world_persistence)
            .await
            .context("Failed to load C++ vehicle accessory rows")?,
    );
    let creature_spawn_store = Arc::new(
        crate::world_reference_catalog::load_world_spawn_id_store_like_cpp(
            &world_reference_catalog_persistence,
            wow_persistence::WorldSpawnCatalogKindLikeCpp::Creature,
        )
        .await
        .context("Failed to load creature spawn ids for C++ ConditionMgr validation")?,
    );
    let gameobject_spawn_store = Arc::new(
        crate::world_reference_catalog::load_world_spawn_id_store_like_cpp(
            &world_reference_catalog_persistence,
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
    // C++ acquisition authority is composed in dependency order: the final
    // SkillLine identities first, then SkillLineAbility/SkillRaceClassInfo
    // with their official/custom overlays and final removals.
    let skill_catalog_hotfix_persistence =
        wow_database::MariaDbSkillCatalogHotfixPersistenceAdapterLikeCpp::new(Arc::clone(
            &hotfix_db,
        ));
    let skill_line_store = Arc::new(
        crate::skill_catalog_hotfix::load_skill_line_store_like_cpp(
            &data_dir,
            &locale,
            &skill_catalog_hotfix_persistence,
            &db2_hotfix_removals,
        )
        .await
        .context("Failed to load effective SkillLine store")?,
    );
    info!(
        "Loaded {} hydrated SkillLine rows and {} effective C++ lookup identities",
        skill_line_store.len(),
        skill_line_store.effective_record_count_like_cpp()
    );
    let skill_store_outcome = crate::skill_catalog_hotfix::load_skill_store_like_cpp(
        &data_dir,
        &locale,
        &skill_catalog_hotfix_persistence,
        &db2_hotfix_removals,
        skill_line_store.as_ref(),
    )
    .await
    .context("Failed to load effective SkillLineAbility/SkillRaceClassInfo stores")?;
    let skill_store_report = &skill_store_outcome.report;
    info!(
        "Loaded {} effective SkillLineAbility rows ({} indexed, {} invalid, {} removed) and {} effective SkillRaceClassInfo rows ({} indexed, {} invalid, {} missing SkillLine, {} removed)",
        skill_store_report.skill_line_ability_effective_rows,
        skill_store_report.skill_line_ability_indexed_rows,
        skill_store_report.skill_line_ability_invalid_rows,
        skill_store_report.skill_line_ability_removed_rows,
        skill_store_report.skill_race_class_info_effective_rows,
        skill_store_report.skill_race_class_info_indexed_rows,
        skill_store_report.skill_race_class_info_invalid_rows,
        skill_store_report.skill_race_class_info_missing_skill_line_rows,
        skill_store_report.skill_race_class_info_removed_rows,
    );
    let skill_store = Arc::new(skill_store_outcome.store);
    let trait_definition_store = Arc::new(
        wow_data::trait_tree::TraitDefinitionStore::load(&data_dir, &locale)
            .context("Failed to load TraitDefinition.db2")?,
    );
    let trait_node_entry_store = Arc::new(
        wow_data::trait_tree::TraitNodeEntryStore::load(&data_dir, &locale)
            .context("Failed to load TraitNodeEntry.db2")?,
    );
    let skill_tiers_store = Arc::new(
        crate::skill_world_rules::load_skill_tiers_store_like_cpp(&skill_world_rules_persistence)
            .await
            .context("Failed to load world.skill_tiers")?,
    );
    let talent_store = Arc::new(
        wow_data::TalentStore::load(&data_dir, &locale).context("Failed to load Talent.db2")?,
    );
    let talent_tab_store = Arc::new(
        wow_data::TalentTabStore::load(&data_dir, &locale)
            .context("Failed to load TalentTab.db2")?,
    );
    let num_talents_at_level_store = Arc::new(
        wow_data::progression_rewards::NumTalentsAtLevelStore::load(&data_dir, &locale)
            .context("Failed to load NumTalentsAtLevel.db2")?,
    );
    let glyph_properties_store = Arc::new(
        wow_data::GlyphPropertiesStore::load(&data_dir, &locale)
            .context("Failed to load GlyphProperties.db2")?,
    );
    info!(
        "Loaded {} talent rows, {} talent tabs, {} talent-level rows, and {} glyph property rows from DB2",
        talent_store.len(),
        talent_tab_store.len(),
        num_talents_at_level_store.len(),
        glyph_properties_store.len()
    );
    let chr_races_store = Arc::new(
        wow_data::character_progression::ChrRacesStore::load(&data_dir, &locale)
            .context("Failed to load ChrRaces.db2")?,
    );
    info!(
        "Loaded {} race rows from ChrRaces.db2",
        chr_races_store.len()
    );
    // [M0.1/#14] ChrClasses powers the class→display-power map (creature stat setup)
    // and the per-class opening cinematic; without it both fall back to hardcoded
    // defaults. C++ sChrClassesStore (DB2Stores.cpp:94).
    let chr_classes_store = Arc::new(
        wow_data::character_progression::ChrClassesStore::load(&data_dir, &locale)
            .context("Failed to load ChrClasses.db2")?,
    );
    let power_type_store = Arc::new(
        crate::static_data_overlay::load_power_type_store_like_cpp(
            &data_dir,
            &locale,
            &static_data_overlay_persistence,
        )
        .await
        .context("Failed to load PowerType.db2 / hotfix rows")?,
    );
    info!(
        "Loaded {} class rows and {} effective power-type rows from DB2/hotfixes",
        chr_classes_store.len(),
        power_type_store.len()
    );
    let chr_model_store = wow_data::character_progression::ChrModelStore::load(&data_dir, &locale)
        .context("Failed to load ChrModel.db2")?;
    let chr_race_x_chr_model_store =
        wow_data::character_progression::ChrRaceXChrModelStore::load(&data_dir, &locale)
            .context("Failed to load ChrRaceXChrModel.db2")?;
    info!(
        "Loaded {} character models and {} race/model links from DB2",
        chr_model_store.len(),
        chr_race_x_chr_model_store.len()
    );
    let creature_family_store = Arc::new(
        wow_data::CreatureFamilyStore::load(&data_dir, &locale)
            .context("Failed to load CreatureFamily.db2")?,
    );
    info!(
        "Loaded {} creature family rows from CreatureFamily.db2",
        creature_family_store.len()
    );
    let spell_levels_store = Arc::new(
        wow_data::SpellLevelsStore::load(&data_dir, &locale)
            .context("Failed to load SpellLevels.db2")?,
    );
    info!(
        "Loaded {} spell level rows from SpellLevels.db2",
        spell_levels_store.len()
    );
    let spell_core_hotfix_persistence =
        wow_database::MariaDbSpellCoreDb2HotfixPersistenceAdapterLikeCpp::new(Arc::clone(
            &hotfix_db,
        ));
    let spell_acquisition_startup_persistence =
        wow_database::MariaDbSpellAcquisitionStartupPersistenceAdapterLikeCpp::new(
            Arc::clone(&hotfix_db),
            Arc::clone(&world_db),
        );
    let (spell_name_store, spell_name_load_report) =
        spell_core_db2_hotfix::load_spell_name_store_like_cpp(
            &data_dir,
            &locale,
            &spell_core_hotfix_persistence,
            &db2_hotfix_removals,
        )
        .await
        .context("Failed to load effective SpellName store")?;
    info!(
        "Loaded {} effective SpellName rows ({} SQL overlay rows; {} removed rows; {} final DB2 removals total)",
        spell_name_store.len(),
        spell_name_load_report.overlay_rows,
        spell_name_load_report.removed_rows,
        db2_hotfix_removals.len()
    );
    let spell_info_key_hotfix_persistence =
        wow_database::MariaDbSpellInfoKeyHotfixPersistenceAdapterLikeCpp::new(Arc::clone(
            &hotfix_db,
        ));
    let spell_store_seed = spell_info_key_hotfix::load_spell_store_seed_like_cpp(
        &data_dir,
        &locale,
        &spell_info_key_hotfix_persistence,
        &spell_name_store,
        &db2_hotfix_removals,
    )
    .await
    .context("Failed to load SpellInfo key authority")?;
    let mut spell_store = spell_core_db2_hotfix::load_spell_store_like_cpp(
        &data_dir,
        &locale,
        spell_store_seed,
        &spell_core_hotfix_persistence,
        &db2_hotfix_removals,
    )
    .await
    .context("Failed to load SpellStore")?;
    info!(
        "Loaded {} hydrated spells and {} exact regular SpellInfo keys from SpellStore",
        spell_store.len(),
        spell_store.spell_info_key_count_like_cpp()
    );
    let pet_levelup_spell_store = Arc::new(wow_data::PetLevelupSpellStoreLikeCpp::load_like_cpp(
        creature_family_store.entries_like_cpp(),
        skill_store.as_ref(),
        |spell_id| {
            let spell = spell_store.get(spell_id)?;
            let spell_id = u32::try_from(spell.spell_id).ok()?;
            let spell_level = spell_levels_store
                .entry_for_spell_difficulty_like_cpp(spell_id, 0)
                .map(|entry| u32::try_from(entry.spell_level).unwrap_or(0))
                .unwrap_or(0);

            Some(wow_data::PetLevelupSpellInfoLikeCpp {
                id: spell_id,
                spell_level,
            })
        },
    ));
    info!(
        "Loaded {} pet levelup spells for {} families",
        pet_levelup_spell_store.count(),
        pet_levelup_spell_store.family_count()
    );
    let pet_default_spell_store = Arc::new(wow_data::PetDefaultSpellStoreLikeCpp::load_like_cpp(
        spell_store
            .iter()
            .map(|spell| wow_data::PetDefaultSpellInfoLikeCpp {
                difficulty_none: true,
                effects: spell
                    .effects()
                    .iter()
                    .map(|effect| wow_data::PetDefaultSpellEffectLikeCpp {
                        effect: effect.effect,
                        misc_value: effect.effect_misc_value_1,
                    })
                    .collect(),
            }),
        creature_template_lifecycle_store
            .entries_like_cpp()
            .map(|template| {
                let mut spells = [0; wow_data::MAX_CREATURE_SPELL_DATA_SLOT_LIKE_CPP];
                spells.copy_from_slice(
                    &template.spells[..wow_data::MAX_CREATURE_SPELL_DATA_SLOT_LIKE_CPP],
                );
                wow_data::PetDefaultSpellCreatureTemplateLikeCpp {
                    entry: template.entry,
                    family: template.family,
                    spells,
                }
            }),
        pet_levelup_spell_store.as_ref(),
    ));
    info!(
        "Loaded {} summonable creature default spell templates",
        pet_default_spell_store.count()
    );
    let spell_category_store = Arc::new(
        spell_core_db2_hotfix::load_spell_category_store_like_cpp(
            &data_dir,
            &locale,
            &spell_core_hotfix_persistence,
            &db2_hotfix_removals,
        )
        .await
        .context("Failed to load effective SpellCategory authority")?,
    );
    info!(
        "Loaded {} spell categories from SpellCategory.db2",
        spell_category_store.len()
    );
    let spell_aura_options_store = Arc::new(
        wow_data::SpellAuraOptionsStore::load(&data_dir, &locale)
            .context("Failed to load SpellAuraOptions.db2")?,
    );
    info!(
        "Loaded {} spell aura options rows",
        spell_aura_options_store.len()
    );
    let spell_aura_restrictions_store = Arc::new(
        spell_core_db2_hotfix::load_spell_aura_restrictions_store_like_cpp(
            &data_dir,
            &locale,
            &spell_core_hotfix_persistence,
            &db2_hotfix_removals,
        )
        .await
        .context("Failed to load effective SpellAuraRestrictions authority")?,
    );
    info!(
        "Loaded {} spell aura restriction rows",
        spell_aura_restrictions_store.len()
    );
    let spell_casting_requirements_store = Arc::new(
        spell_core_db2_hotfix::load_spell_casting_requirements_store_like_cpp(
            &data_dir,
            &locale,
            &spell_core_hotfix_persistence,
            &db2_hotfix_removals,
        )
        .await
        .context("Failed to load effective SpellCastingRequirements authority")?,
    );
    info!(
        "Loaded {} spell casting requirement rows",
        spell_casting_requirements_store.len()
    );
    let spell_class_options_store = Arc::new(
        wow_data::SpellClassOptionsStore::load(&data_dir, &locale)
            .context("Failed to load SpellClassOptions.db2")?,
    );
    info!(
        "Loaded {} spell class options rows",
        spell_class_options_store.len()
    );
    let spell_equipped_items_store = Arc::new(
        spell_core_db2_hotfix::load_spell_equipped_items_store_like_cpp(
            &data_dir,
            &locale,
            &spell_core_hotfix_persistence,
            &db2_hotfix_removals,
        )
        .await
        .context("Failed to load effective SpellEquippedItems authority")?,
    );
    info!(
        "Loaded {} spell equipped items rows",
        spell_equipped_items_store.len()
    );
    let spell_target_restrictions_store = Arc::new(
        spell_core_db2_hotfix::load_spell_target_restrictions_store_like_cpp(
            &data_dir,
            &locale,
            &spell_core_hotfix_persistence,
            &db2_hotfix_removals,
        )
        .await
        .context("Failed to load effective SpellTargetRestrictions authority")?,
    );
    info!(
        "Loaded {} spell target restriction rows",
        spell_target_restrictions_store.len()
    );
    let spell_misc_store = Arc::new(
        spell_core_db2_hotfix::load_spell_misc_store_like_cpp(
            &data_dir,
            &locale,
            &spell_core_hotfix_persistence,
            &db2_hotfix_removals,
        )
        .await
        .context("Failed to load effective SpellMisc authority")?,
    );
    info!(
        "Loaded {} effective spell misc rows",
        spell_misc_store.len()
    );
    let pet_family_spell_store = Arc::new(wow_data::PetFamilySpellStoreLikeCpp::load_like_cpp(
        skill_store.as_ref(),
        creature_family_store.entries_like_cpp(),
        spell_levels_store
            .entries_like_cpp()
            .map(|entry| wow_data::PetFamilySpellLevelLikeCpp {
                spell_id: i32::try_from(entry.spell_id).unwrap_or(0),
                difficulty_id: u32::from(entry.difficulty_id),
                spell_level: entry.spell_level,
            }),
        |spell_id| {
            let spell = spell_store.get(spell_id)?;
            let spell_id = u32::try_from(spell.spell_id).ok()?;
            Some(wow_data::PetFamilySpellInfoLikeCpp {
                id: spell_id,
                is_passive: spell_misc_store.is_passive_like_cpp(spell_id),
            })
        },
    ));
    info!(
        "Loaded {} pet family passive spells for {} families",
        pet_family_spell_store.spell_count(),
        pet_family_spell_store.family_count()
    );
    let spell_procs_per_minute_store = Arc::new(
        wow_data::SpellProcsPerMinuteStore::load(&data_dir, &locale)
            .context("Failed to load SpellProcsPerMinute.db2")?,
    );
    info!(
        "Loaded {} spell procs-per-minute rows",
        spell_procs_per_minute_store.len()
    );
    let spell_duration_store = Arc::new(
        spell_core_db2_hotfix::load_spell_duration_store_like_cpp(
            &data_dir,
            &locale,
            &spell_core_hotfix_persistence,
            &db2_hotfix_removals,
        )
        .await
        .context("Failed to load effective SpellDuration authority")?,
    );
    info!("Loaded {} spell duration rows", spell_duration_store.len());
    let spell_cooldowns_store = Arc::new(
        spell_core_db2_hotfix::load_spell_cooldowns_store_like_cpp(
            &data_dir,
            &locale,
            &spell_core_hotfix_persistence,
            &db2_hotfix_removals,
        )
        .await
        .context("Failed to load effective SpellCooldowns authority")?,
    );
    info!("Loaded {} spell cooldown rows", spell_cooldowns_store.len());
    let spell_shapeshift_form_store = Arc::new(
        wow_data::SpellShapeshiftFormStore::load(&data_dir, &locale)
            .context("Failed to load SpellShapeshiftForm.db2")?,
    );
    info!(
        "Loaded {} spell shapeshift form rows",
        spell_shapeshift_form_store.len()
    );
    let creature_addon_store = Arc::new(
        crate::world_object_catalog::load_creature_addons_like_cpp(
            &world_object_catalog_persistence,
            creature_template_lifecycle_store.as_ref(),
            creature_spawn_store.as_ref(),
            creature_display_info_store.as_ref(),
            emotes_store.as_ref(),
            anim_kit_store.as_ref(),
            &spell_store,
            spell_misc_store.as_ref(),
            spell_duration_store.as_ref(),
        )
        .await
        .context("Failed to load represented creature_addon / creature_template_addon rows for C++ Creature::LoadCreaturesAddon")?,
    );
    info!(
        "Loaded {} represented creature addon rows",
        creature_addon_store.len()
    );
    let active_event_store = Arc::new(
        crate::world_reference_catalog::load_world_id_store_like_cpp(
            &world_reference_catalog_persistence,
            wow_persistence::WorldObjectIdCatalogKindLikeCpp::GameEvent,
        )
        .await
        .context("Failed to load game_event ids for C++ ConditionMgr validation")?,
    );
    let world_state_store = Arc::new(
        crate::world_reference_catalog::load_world_id_store_like_cpp(
            &world_reference_catalog_persistence,
            wow_persistence::WorldObjectIdCatalogKindLikeCpp::WorldState,
        )
        .await
        .context("Failed to load world_state ids for C++ ConditionMgr validation")?,
    );
    info!(
        "Loaded condition validation world id stores: {} valid game events, {} world states",
        active_event_store.len(),
        world_state_store.len()
    );
    let trainer_store = Arc::new(
        crate::world_reference_catalog::load_world_id_store_like_cpp(
            &world_reference_catalog_persistence,
            wow_persistence::WorldObjectIdCatalogKindLikeCpp::Trainer,
        )
        .await
        .context("Failed to load trainer ids for C++ ConditionMgr validation")?,
    );
    info!(
        "Loaded condition validation trainer id store: {} trainers",
        trainer_store.len()
    );
    let curve_store = Arc::new(
        wow_data::progression_rewards::CurveStore::load(&data_dir, &locale)
            .context("Failed to load Curve.db2 for C++ curve validation")?,
    );
    let curve_point_store = Arc::new(
        wow_data::progression_rewards::CurvePointStore::load(&data_dir, &locale)
            .context("Failed to load CurvePoint.db2 for C++ curve evaluation")?,
    );
    let scaling_stat_distribution_store = Arc::new(
        wow_data::progression_rewards::ScalingStatDistributionStore::load(&data_dir, &locale)
            .context(
                "Failed to load ScalingStatDistribution.db2 — check DataDir and DBC.Locale config",
            )?,
    );
    let scaling_stat_values_store = Arc::new(
        wow_data::progression_rewards::ScalingStatValuesStore::load(&data_dir, &locale).context(
            "Failed to load ScalingStatValues.db2 — check DataDir and DBC.Locale config",
        )?,
    );
    info!(
        "Loaded {} curves, {} curve points, {} scaling-stat distributions, and {} scaling-stat values from DB2",
        curve_store.len(),
        curve_point_store.len(),
        scaling_stat_distribution_store.len(),
        scaling_stat_values_store.len()
    );
    let area_trigger_template_persistence =
        wow_database::MariaDbAreaTriggerTemplateCatalogPersistenceAdapterLikeCpp::new(Arc::clone(
            &world_db,
        ));
    let area_trigger_template_outcome =
        crate::area_trigger_template_catalog::load_area_trigger_template_store_like_cpp(
            &area_trigger_template_persistence,
            &world_safe_loc_store,
            |id| curve_store.get(id).is_some(),
            |name| script_name_interner.get_script_id_like_cpp(name, true),
        )
        .await
        .context("Failed to load C++ AreaTriggerDataStore template/create-properties rows")?;
    for (area_trigger_id, action_type, param) in &area_trigger_template_outcome
        .report
        .skipped_actions_invalid_action_type
    {
        tracing::error!(
            target: "sql.sql",
            "Table `areatrigger_template_actions` has invalid ActionType {} for AreaTriggerId ({},{}) and Param {}",
            action_type,
            area_trigger_id.id,
            u32::from(area_trigger_id.is_custom),
            param
        );
    }
    for (area_trigger_id, target_type, param) in &area_trigger_template_outcome
        .report
        .skipped_actions_invalid_target_type
    {
        tracing::error!(
            target: "sql.sql",
            "Table `areatrigger_template_actions` has invalid TargetType {} for AreaTriggerId ({},{}) and Param {}",
            target_type,
            area_trigger_id.id,
            u32::from(area_trigger_id.is_custom),
            param
        );
    }
    for (area_trigger_id, param) in &area_trigger_template_outcome
        .report
        .skipped_actions_invalid_teleport_world_safe_loc
    {
        tracing::error!(
            target: "sql.sql",
            "Table `areatrigger_template_actions` has invalid entry for AreaTriggerId ({},{}) with TargetType=Teleport and Param ({}) not a valid world safe loc entry",
            area_trigger_id.id,
            u32::from(area_trigger_id.is_custom),
            param
        );
    }
    for (create_properties_id, idx) in &area_trigger_template_outcome
        .report
        .invalid_partial_target_vertices
    {
        tracing::error!(
            target: "sql.sql",
            "Table `areatrigger_create_properties_polygon_vertex` has listed invalid target vertices (AreaTriggerCreatePropertiesId: ({},{}), Index: {}).",
            create_properties_id.id,
            u32::from(create_properties_id.is_custom),
            idx
        );
    }
    for (create_properties_id, area_trigger_id) in &area_trigger_template_outcome
        .report
        .skipped_create_properties_invalid_template
    {
        tracing::error!(
            target: "sql.sql",
            "Table `areatrigger_create_properties` references invalid AreaTrigger (Id: {}, IsCustom: {}) for AreaTriggerCreatePropertiesId (Id: {}, IsCustom: {})",
            area_trigger_id.id,
            u32::from(area_trigger_id.is_custom),
            create_properties_id.id,
            u32::from(create_properties_id.is_custom)
        );
    }
    for (create_properties_id, shape) in &area_trigger_template_outcome
        .report
        .skipped_create_properties_invalid_shape
    {
        tracing::error!(
            target: "sql.sql",
            "Table `areatrigger_create_properties` has listed AreaTriggerCreatePropertiesId (Id: {}, IsCustom: {}) with invalid shape {}.",
            create_properties_id.id,
            u32::from(create_properties_id.is_custom),
            shape
        );
    }
    for (area_trigger_id, create_properties_id, curve_field, curve_id) in
        &area_trigger_template_outcome
            .report
            .corrected_create_properties_invalid_curves
    {
        let curve_name = match curve_field {
            wow_data::AreaTriggerCurveFieldLikeCpp::Move => "MoveCurveId",
            wow_data::AreaTriggerCurveFieldLikeCpp::Scale => "ScaleCurveId",
            wow_data::AreaTriggerCurveFieldLikeCpp::Morph => "MorphCurveId",
            wow_data::AreaTriggerCurveFieldLikeCpp::Facing => "FacingCurveId",
        };
        tracing::error!(
            target: "sql.sql",
            "Table `areatrigger_create_properties` has listed AreaTrigger (Id: {}, IsCustom: {}) for AreaTriggerCreatePropertiesId (Id: {}, IsCustom: {}) with invalid {} ({}), set to 0!",
            area_trigger_id.id,
            u32::from(area_trigger_id.is_custom),
            create_properties_id.id,
            u32::from(create_properties_id.is_custom),
            curve_name,
            curve_id
        );
    }
    for create_properties_id in &area_trigger_template_outcome
        .report
        .invalid_polygon_target_vertex_counts
    {
        tracing::error!(
            target: "sql.sql",
            "Table `areatrigger_create_properties_polygon_vertex` has invalid target vertices, either all or none vertices must have a corresponding target vertex (AreaTriggerCreatePropertiesId: (Id: {}, IsCustom: {})).",
            create_properties_id.id,
            u32::from(create_properties_id.is_custom)
        );
    }
    for create_properties_id in &area_trigger_template_outcome
        .report
        .skipped_orbit_invalid_create_properties
    {
        tracing::error!(
            target: "sql.sql",
            "Table `areatrigger_create_properties_orbit` reference invalid AreaTriggerCreatePropertiesId: (Id: {}, IsCustom: {})",
            create_properties_id.id,
            u32::from(create_properties_id.is_custom)
        );
    }
    for (create_properties_id, float_field, value) in &area_trigger_template_outcome
        .report
        .corrected_orbit_invalid_floats
    {
        let float_name = match float_field {
            wow_data::AreaTriggerOrbitFloatFieldLikeCpp::Radius => "Radius",
            wow_data::AreaTriggerOrbitFloatFieldLikeCpp::BlendFromRadius => "BlendFromRadius",
            wow_data::AreaTriggerOrbitFloatFieldLikeCpp::InitialAngle => "InitialAngle",
            wow_data::AreaTriggerOrbitFloatFieldLikeCpp::ZOffset => "ZOffset",
        };
        tracing::error!(
            target: "sql.sql",
            "Table `areatrigger_create_properties_orbit` has listed areatrigger (AreaTriggerCreatePropertiesId: {}, IsCustom: {}) with invalid {} ({}), set to 0!",
            create_properties_id.id,
            u32::from(create_properties_id.is_custom),
            float_name,
            value
        );
    }
    let area_trigger_template_report = area_trigger_template_outcome.report;
    let area_trigger_template_store = Arc::new(area_trigger_template_outcome.store);
    info!(
        "Loaded {} C++ area-trigger templates, {} create properties, {} orbit infos, {} actions, {} polygon vertices ({} targets), and {} spline points from {} template rows / {} create-property rows / {} orbit rows / {} action rows / {} polygon rows / {} spline rows ({} invalid rows skipped; spawns pending)",
        area_trigger_template_report.loaded_templates,
        area_trigger_template_report.loaded_create_properties,
        area_trigger_template_report.loaded_orbit_infos,
        area_trigger_template_report.loaded_actions,
        area_trigger_template_report.loaded_polygon_vertices,
        area_trigger_template_report.loaded_polygon_target_vertices,
        area_trigger_template_report.loaded_spline_points,
        area_trigger_template_report.template_rows_seen,
        area_trigger_template_report.create_properties_rows_seen,
        area_trigger_template_report.orbit_rows_seen,
        area_trigger_template_report.action_rows_seen,
        area_trigger_template_report.polygon_vertex_rows_seen,
        area_trigger_template_report.spline_point_rows_seen,
        area_trigger_template_report
            .skipped_actions_invalid_action_type
            .len()
            + area_trigger_template_report
                .skipped_actions_invalid_target_type
                .len()
            + area_trigger_template_report
                .skipped_actions_invalid_teleport_world_safe_loc
                .len()
            + area_trigger_template_report
                .invalid_partial_target_vertices
                .len()
            + area_trigger_template_report
                .skipped_create_properties_invalid_template
                .len()
            + area_trigger_template_report
                .skipped_create_properties_invalid_shape
                .len()
            + area_trigger_template_report
                .corrected_create_properties_invalid_curves
                .len()
            + area_trigger_template_report
                .invalid_polygon_target_vertex_counts
                .len()
            + area_trigger_template_report
                .skipped_orbit_invalid_create_properties
                .len()
            + area_trigger_template_report
                .corrected_orbit_invalid_floats
                .len()
    );
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
        wow_database::MariaDbLfgDungeonsHotfixPersistenceAdapterLikeCpp::new(Arc::clone(
            &hotfix_db,
        ));
    let lfg_dungeons_store = Arc::new(
        lfg_dungeons_hotfix::load_lfg_dungeons_like_cpp(
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
        crate::world_object_catalog::load_creature_equipment_like_cpp(
            &world_object_catalog_persistence,
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

    let game_event_persistence: Arc<dyn wow_persistence::GameEventPersistencePortLikeCpp> =
        Arc::new(
            wow_database::MariaDbGameEventPersistenceAdapterLikeCpp::new(Arc::clone(&char_db)),
        );
    let game_event_world_catalog: Arc<
        dyn wow_persistence::GameEventWorldCatalogPersistencePortLikeCpp,
    > = Arc::new(
        wow_database::MariaDbGameEventWorldCatalogPersistenceAdapterLikeCpp::new(Arc::clone(
            &world_db,
        )),
    );
    let canonical_spawn_catalog: Arc<
        dyn wow_persistence::CanonicalSpawnCatalogPersistencePortLikeCpp,
    > = Arc::new(
        wow_database::MariaDbCanonicalSpawnCatalogPersistenceAdapterLikeCpp::new(Arc::clone(
            &world_db,
        )),
    );
    let (canonical_spawn_metadata, canonical_spawn_report) =
        spawn_store_loader::load_canonical_spawn_store_like_cpp(
            canonical_spawn_catalog.as_ref(),
            game_event_persistence.as_ref(),
            game_event_world_catalog.as_ref(),
            &map_store,
            &map_difficulty_store,
            &spawn_group_store,
            creature_equipment_store.as_ref(),
            area_trigger_template_store.as_ref(),
            |spell_id| spell_store.get(spell_id as i32).is_some(),
            |name| script_name_interner.get_script_id_like_cpp(name, true),
        )
        .await
        .context("Failed to load canonical SpawnStore metadata from world DB")?;
    info!(
        "Loaded canonical SpawnStore metadata: creatures rows={} indexed={} event-managed={} empty-difficulty={} missing-map={}; formations rows={} loaded={} missing-leader={} missing-member={} duplicate-member={} pruned-missing-leader-self={}; gameobjects rows={} indexed={} event-managed={} empty-difficulty={} missing-map={}; areatriggers rows={} indexed={} empty-difficulty={} missing-map={} invalid-create-properties={} flags={} curves={} time={} orbit={} splines={} invalid-spell={}; poolmgr templates rows={} loaded={} creature-members loaded={}/{} gameobject-members loaded={}/{} pool-members loaded={}/{} relation-removals={} map-mismatches={} circular={} empty={} missing-map={} autospawn loaded={}/{} skipped-empty={} skipped-broken={} skipped-child={}; spawn-group rows={} assigned={} missing-spawn={} invalid-type={} missing-group={} map-mismatch={} duplicate={}; represented validations skipped: creature={} gameobject={} areatrigger={}",
        canonical_spawn_report.creature.rows,
        canonical_spawn_report.creature.indexed,
        canonical_spawn_report.creature.skipped_event,
        canonical_spawn_report.creature.skipped_empty_difficulties,
        canonical_spawn_report.creature.skipped_missing_map,
        canonical_spawn_report.creature_formations.rows,
        canonical_spawn_report.creature_formations.loaded,
        canonical_spawn_report
            .creature_formations
            .skipped_missing_leader,
        canonical_spawn_report
            .creature_formations
            .skipped_missing_member,
        canonical_spawn_report
            .creature_formations
            .duplicate_member_ignored,
        canonical_spawn_report
            .creature_formations
            .removed_missing_leader_self,
        canonical_spawn_report.gameobject.rows,
        canonical_spawn_report.gameobject.indexed,
        canonical_spawn_report.gameobject.skipped_event,
        canonical_spawn_report.gameobject.skipped_empty_difficulties,
        canonical_spawn_report.gameobject.skipped_missing_map,
        canonical_spawn_report.area_trigger.rows,
        canonical_spawn_report.area_trigger.indexed,
        canonical_spawn_report
            .area_trigger
            .skipped_empty_difficulties,
        canonical_spawn_report.area_trigger.skipped_missing_map,
        canonical_spawn_report
            .area_trigger
            .skipped_invalid_create_properties
            .len(),
        canonical_spawn_report
            .area_trigger
            .skipped_nonzero_create_properties_flags
            .len(),
        canonical_spawn_report
            .area_trigger
            .skipped_create_properties_curves
            .len(),
        canonical_spawn_report
            .area_trigger
            .skipped_create_properties_time_to_target
            .len(),
        canonical_spawn_report
            .area_trigger
            .skipped_create_properties_orbit
            .len(),
        canonical_spawn_report
            .area_trigger
            .skipped_create_properties_splines
            .len(),
        canonical_spawn_report
            .area_trigger
            .corrected_invalid_spell_for_visuals
            .len(),
        canonical_spawn_report.pool_mgr.template_rows,
        canonical_spawn_report.pool_mgr.templates_loaded,
        canonical_spawn_report.pool_mgr.creature_members.loaded,
        canonical_spawn_report.pool_mgr.creature_members.rows,
        canonical_spawn_report.pool_mgr.gameobject_members.loaded,
        canonical_spawn_report.pool_mgr.gameobject_members.rows,
        canonical_spawn_report.pool_mgr.pool_members.loaded,
        canonical_spawn_report.pool_mgr.pool_members.rows,
        canonical_spawn_report.pool_mgr.relation_removals,
        canonical_spawn_report.pool_mgr.map_mismatches,
        canonical_spawn_report.pool_mgr.circular_relations,
        canonical_spawn_report.pool_mgr.empty_pools,
        canonical_spawn_report.pool_mgr.missing_map_after_non_empty,
        canonical_spawn_report.pool_mgr.autospawn_loaded,
        canonical_spawn_report.pool_mgr.autospawn_rows,
        canonical_spawn_report.pool_mgr.autospawn_skipped_empty,
        canonical_spawn_report.pool_mgr.autospawn_skipped_broken,
        canonical_spawn_report.pool_mgr.autospawn_skipped_child,
        canonical_spawn_report.spawn_group_rows,
        canonical_spawn_report.spawn_group_apply.assigned,
        canonical_spawn_report.spawn_group_apply.missing_spawn,
        canonical_spawn_report.spawn_group_apply.invalid_type,
        canonical_spawn_report.spawn_group_apply.missing_group,
        canonical_spawn_report.spawn_group_apply.map_mismatch,
        canonical_spawn_report
            .spawn_group_apply
            .duplicate_spawn_group,
        canonical_spawn_report.creature.validation_skipped,
        canonical_spawn_report.gameobject.validation_skipped,
        canonical_spawn_report.area_trigger.validation_skipped,
    );
    let mut script_name_interner = Arc::new(script_name_interner);
    info!(
        "Built C++ ScriptNameContainer core from loaded template/scene/area-trigger/spawn stores: {} names ({} DB-bound)",
        script_name_interner.len_like_cpp(),
        script_name_interner.all_db_script_names_like_cpp().len()
    );
    let respawn_persistence: Arc<dyn RespawnPersistencePortLikeCpp> = Arc::new(
        wow_database::MariaDbRespawnPersistenceAdapterLikeCpp::new(Arc::clone(&char_db)),
    );
    let (persisted_respawn_times, persisted_respawn_report) =
        load_persisted_respawn_times_like_cpp(
            respawn_persistence.as_ref(),
            &canonical_spawn_metadata,
        )
        .await
        .context("Failed to load persisted respawn times from character database")?;
    let persisted_respawn_times = Arc::new(persisted_respawn_times);
    info!(
        "Loaded persisted C++ respawn timers: rows={} loaded={} maps={} timers={} invalid-type={} unsupported-areatrigger={} missing-spawn-metadata={}",
        persisted_respawn_report.rows,
        persisted_respawn_report.loaded,
        persisted_respawn_times.maps_len(),
        persisted_respawn_times.respawns_len(),
        persisted_respawn_report.invalid_type,
        persisted_respawn_report.unsupported_area_trigger,
        persisted_respawn_report.missing_spawn_metadata,
    );
    let canonical_spawn_metadata: SharedCanonicalSpawnMetadataLikeCpp =
        Arc::new(Mutex::new(canonical_spawn_metadata));
    let world_state_startup: Arc<dyn wow_persistence::WorldStateStartupPersistencePortLikeCpp> =
        Arc::new(
            wow_database::MariaDbWorldStateStartupPersistenceAdapterLikeCpp::new(
                Arc::clone(&world_db),
                Arc::clone(&char_db),
            ),
        );
    let (world_state_mgr, world_state_mgr_report) =
        spawn_store_loader::load_world_state_mgr_like_cpp(
            world_state_startup.as_ref(),
            &map_store,
            &area_table_store,
        )
        .await
        .context("Failed to load C++ WorldStateMgr startup state")?;
    info!(
        "Loaded C++ WorldStateMgr startup state: template rows={} loaded={} skipped-map-list={} skipped-area-list={} realm-area-ignored={} saved rows={} applied={} skipped-unknown={}",
        world_state_mgr_report.template_rows,
        world_state_mgr_report.templates_loaded,
        world_state_mgr_report.skipped_invalid_map_list,
        world_state_mgr_report.skipped_invalid_area_list,
        world_state_mgr_report.realm_area_requirements_ignored,
        world_state_mgr_report.saved_rows,
        world_state_mgr_report.saved_applied,
        world_state_mgr_report.saved_skipped_unknown,
    );
    let world_state_mgr: SharedWorldStateMgrLikeCpp = Arc::new(Mutex::new(world_state_mgr));

    let mount_catalog_persistence = wow_database::MariaDbMountCatalogPersistenceAdapterLikeCpp::new(
        Arc::clone(&hotfix_db),
        Arc::clone(&world_db),
    );
    let (mount_store, mount_hotfix_rows) = crate::mount_catalog::load_mount_store_like_cpp(
        &data_dir,
        &locale,
        &mount_catalog_persistence,
    )
    .await
    .context("Failed to load Mount.db2 / hotfix rows")?;
    if mount_hotfix_rows != 0 {
        info!("Loaded {mount_hotfix_rows} Mount hotfix rows");
    }
    let mount_store = Arc::new(mount_store);
    info!("Loaded {} mounts from Mount.db2", mount_store.len());
    let mount_definition_store = Arc::new(
        crate::mount_catalog::load_mount_definition_store_like_cpp(
            &mount_store,
            &mount_catalog_persistence,
        )
        .await
        .context("Failed to load mount_definitions")?,
    );
    info!(
        "Loaded {} faction-specific mount definitions from mount_definitions",
        mount_definition_store.len()
    );
    let (mount_capability_store, mount_capability_hotfix_rows) =
        crate::mount_catalog::load_mount_capability_store_like_cpp(
            &data_dir,
            &locale,
            &mount_catalog_persistence,
        )
        .await
        .context("Failed to load MountCapability.db2 / hotfix rows")?;
    if mount_capability_hotfix_rows != 0 {
        info!("Loaded {mount_capability_hotfix_rows} MountCapability hotfix rows");
    }
    let mount_capability_store = Arc::new(mount_capability_store);
    info!(
        "Loaded {} mount capabilities from MountCapability.db2",
        mount_capability_store.len()
    );
    let (mount_type_x_capability_store, mount_type_x_capability_hotfix_rows) =
        crate::mount_catalog::load_mount_type_x_capability_store_like_cpp(
            &data_dir,
            &locale,
            &mount_catalog_persistence,
        )
        .await
        .context("Failed to load MountTypeXCapability.db2 / hotfix rows")?;
    if mount_type_x_capability_hotfix_rows != 0 {
        info!("Loaded {mount_type_x_capability_hotfix_rows} MountTypeXCapability hotfix rows");
    }
    let mount_type_x_capability_store = Arc::new(mount_type_x_capability_store);
    info!(
        "Loaded {} mount type capability rows from MountTypeXCapability.db2",
        mount_type_x_capability_store.len()
    );
    let (mount_x_display_store, mount_x_display_hotfix_rows) =
        crate::mount_catalog::load_mount_x_display_store_like_cpp(
            &data_dir,
            &locale,
            &mount_catalog_persistence,
        )
        .await
        .context("Failed to load MountXDisplay.db2 / hotfix rows")?;
    if mount_x_display_hotfix_rows != 0 {
        info!("Loaded {mount_x_display_hotfix_rows} MountXDisplay hotfix rows");
    }
    let mount_x_display_store = Arc::new(mount_x_display_store);
    info!(
        "Loaded {} mount display rows from MountXDisplay.db2",
        mount_x_display_store.len()
    );
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
    let world_state_expression_store = Arc::new(
        wow_data::WorldStateExpressionStore::load(&data_dir, &locale).context(
            "Failed to load WorldStateExpression.db2 — check DataDir and DBC.Locale config",
        )?,
    );
    let conversation_line_store = Arc::new(
        wow_data::Db2IdStore::load(&data_dir, &locale, "ConversationLine.db2")
            .context("Failed to load ConversationLine.db2 — check DataDir and DBC.Locale config")?,
    );
    let conversation_line_template_store = Arc::new(
        crate::world_reference_catalog::load_filtering_world_id_store_like_cpp(
            &world_reference_catalog_persistence,
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

    // Load ItemSearchName.db2 for CollectionMgr::CanAddAppearance item-name existence gate.
    let item_search_name_store = Arc::new(
        wow_data::ItemSearchNameStore::load(&data_dir, &locale)
            .context("Failed to load ItemSearchName.db2 — check DataDir and DBC.Locale config")?,
    );
    info!(
        "Loaded {} item search-name rows from ItemSearchName.db2",
        item_search_name_store.len()
    );
    let trinity_string_store = Arc::new(
        crate::world_auxiliary_catalog::load_trinity_strings_like_cpp(
            &world_auxiliary_catalog_persistence,
        )
        .await
        .context("Failed to load C++ trinity_string rows")?,
    );
    info!(
        "Loaded {} C++ trinity_string rows",
        trinity_string_store.len()
    );

    // Load battle-pet stat DB2 stores used by BattlePet::CalculateStats.
    let battle_pet_breed_quality_store = Arc::new(
        wow_data::BattlePetBreedQualityStore::load(&data_dir, &locale).context(
            "Failed to load BattlePetBreedQuality.db2 — check DataDir and DBC.Locale config",
        )?,
    );
    let battle_pet_breed_state_store = Arc::new(
        wow_data::BattlePetBreedStateStore::load(&data_dir, &locale).context(
            "Failed to load BattlePetBreedState.db2 — check DataDir and DBC.Locale config",
        )?,
    );
    let battle_pet_species_entry_store = Arc::new(
        wow_data::BattlePetSpeciesStore::load(&data_dir, &locale)
            .context("Failed to load BattlePetSpecies.db2 — check DataDir and DBC.Locale config")?,
    );
    let battle_pet_species_state_store = Arc::new(
        wow_data::BattlePetSpeciesStateStore::load(&data_dir, &locale).context(
            "Failed to load BattlePetSpeciesState.db2 — check DataDir and DBC.Locale config",
        )?,
    );
    let battle_pet_xp_game_table = Arc::new(
        wow_data::BattlePetXpGameTableLikeCpp::load(&data_dir)
            .context("Failed to load gt/BattlePetXP.txt — check DataDir config")?,
    );
    let combat_ratings_game_table = Arc::new(
        wow_data::CombatRatingsGameTableLikeCpp::load(&data_dir)
            .context("Failed to load gt/CombatRatings.txt - check DataDir config")?,
    );
    info!(
        "Loaded battle-pet stat DB2 stores: {} quality rows, {} breed-state rows, {} species rows, {} species-state rows; BattlePetXP rows={}; CombatRatings rows={}",
        battle_pet_breed_quality_store.len(),
        battle_pet_breed_state_store.len(),
        battle_pet_species_entry_store.len(),
        battle_pet_species_state_store.len(),
        battle_pet_xp_game_table.len(),
        combat_ratings_game_table.len()
    );

    let shield_block_regular_game_table = Arc::new(
        wow_data::ShieldBlockRegularGameTableLikeCpp::load(&data_dir)
            .context("Failed to load gt/ShieldBlockRegular.txt - check DataDir config")?,
    );
    info!(
        "Loaded ShieldBlockRegular game table: {} rows",
        shield_block_regular_game_table.len()
    );

    // Load TransmogSet.db2 and TransmogSetItem.db2 for DB2Manager transmog indexes.
    let transmog_set_store = Arc::new(
        wow_data::TransmogSetStore::load(&data_dir, &locale)
            .context("Failed to load TransmogSet.db2 — check DataDir and DBC.Locale config")?,
    );
    info!(
        "Loaded {} transmog sets from TransmogSet.db2",
        transmog_set_store.len()
    );
    let transmog_set_item_store = Arc::new(
        wow_data::TransmogSetItemStore::load_with_sets(&data_dir, &locale, &transmog_set_store)
            .context("Failed to load TransmogSetItem.db2 — check DataDir and DBC.Locale config")?,
    );
    info!(
        "Loaded {} transmog set items from TransmogSetItem.db2",
        transmog_set_item_store.len()
    );

    let player_create_taxi_path_store = wow_data::TaxiPathStore::load(&data_dir, &locale)
        .context("Failed to load TaxiPath.db2 for C++ playercreateinfo")?;
    let player_create_taxi_path_node_store = wow_data::TaxiPathNodeStore::load(&data_dir, &locale)
        .context("Failed to load TaxiPathNode.db2 for C++ playercreateinfo")?;
    let player_create_info_store = Arc::new(
        crate::player_creation_catalog::load_player_create_info_store_like_cpp(
            &player_creation_catalog_persistence,
            &map_store,
            &chr_races_store,
            &chr_classes_store,
            &chr_model_store,
            &chr_race_x_chr_model_store,
            &gameobject_template_lifecycle_store,
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
        crate::player_base_stats::load_player_base_stats_like_cpp(
            &player_base_stats_persistence,
            &data_dir,
            world_config_u8(&world_configs, "CONFIG_MAX_PLAYER_LEVEL", 80),
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
        crate::player_creation_catalog::load_player_create_cast_spell_store_like_cpp(
            &player_creation_catalog_persistence,
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
        crate::player_creation_catalog::load_player_create_custom_spell_store_like_cpp(
            &player_creation_catalog_persistence,
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

    let quest_item_catalog_persistence =
        wow_database::MariaDbQuestItemCatalogPersistenceAdapterLikeCpp::new(Arc::clone(&world_db));
    let (gameobject_quest_item_store, creature_quest_item_store) =
        crate::quest_item_catalog::load_quest_item_catalogs_like_cpp(
            &quest_item_catalog_persistence,
            |entry| gameobject_template_lifecycle_store.get(entry).is_some(),
            |entry| creature_template_lifecycle_store.get(entry).is_some(),
            |item_id| item_stats_store.sparse_template(item_id).is_some(),
        )
        .await?;
    let gameobject_quest_item_store = Arc::new(gameobject_quest_item_store);
    let _creature_quest_item_store = Arc::new(creature_quest_item_store);

    let world_query_catalog_persistence =
        wow_database::world_query_catalog_adapter::MariaDbWorldQueryCatalogPersistenceAdapterLikeCpp::new(
            Arc::clone(&world_db),
        );
    let (creature_query_catalog, gameobject_query_catalog, page_text_catalog) =
        crate::world_query_catalog::load_like_cpp(&world_query_catalog_persistence)
            .await
            .context("Failed to load immutable C++ ObjectMgr query catalogs")?;
    let object_mgr_catalogs = Arc::new(wow_world::session::ObjectMgrCatalogsLikeCpp {
        creature: Arc::new(creature_query_catalog),
        gameobject: Arc::new(gameobject_query_catalog),
        gameobject_quest_items: gameobject_quest_item_store,
        page_text: Arc::new(page_text_catalog),
        gameobject_lifecycle: Arc::clone(&gameobject_template_lifecycle_store),
    });
    info!(
        creatures = object_mgr_catalogs.creature.len(),
        gameobjects = object_mgr_catalogs.gameobject.len(),
        pages = object_mgr_catalogs.page_text.len(),
        "Loaded immutable C++ ObjectMgr query capability"
    );

    // C++ global DB2 stores used by Item::CalculateDurabilityRepairCost.
    let durability_costs_store = Arc::new(
        wow_data::DurabilityCostsStore::load(&data_dir, &locale)
            .context("Failed to load DurabilityCosts.db2 — check DataDir and DBC.Locale config")?,
    );
    info!(
        "Loaded {} durability cost rows from DurabilityCosts.db2",
        durability_costs_store.len()
    );

    let durability_quality_store = Arc::new(
        wow_data::DurabilityQualityStore::load(&data_dir, &locale).context(
            "Failed to load DurabilityQuality.db2 — check DataDir and DBC.Locale config",
        )?,
    );
    info!(
        "Loaded {} durability quality rows from DurabilityQuality.db2",
        durability_quality_store.len()
    );

    let item_effect_store = Arc::new(
        wow_data::ItemEffectStore::load(&data_dir, &locale)
            .context("Failed to load ItemEffect.db2 — check DataDir and DBC.Locale config")?,
    );
    info!(
        "Loaded {} item effects from ItemEffect.db2",
        item_effect_store.len()
    );

    // Load Lock.db2 for C++ sLockStore existence checks during CMSG_OPEN_ITEM.
    let lock_store = Arc::new(
        wow_data::LockStore::load(&data_dir, &locale)
            .context("Failed to load Lock.db2 — check DataDir and DBC.Locale config")?,
    );
    info!("Loaded {} locks from Lock.db2", lock_store.len());

    // Load ItemRandomSuffix.db2 for C++ ApplyEnchantment random suffix amount resolution.
    let item_random_suffix_store = Arc::new(
        wow_data::ItemRandomSuffixStore::load(&data_dir, &locale)
            .context("Failed to load ItemRandomSuffix.db2 — check DataDir and DBC.Locale config")?,
    );
    info!(
        "Loaded {} item random suffixes from ItemRandomSuffix.db2",
        item_random_suffix_store.len()
    );

    // Load ItemRandomProperties.db2 and RandPropPoints.db2 plus the world-table
    // random enchantment groups for C++ ItemEnchantmentMgr::GenerateRandomProperties.
    let item_random_properties_store = Arc::new(
        wow_data::ItemRandomPropertiesStore::load(&data_dir, &locale).context(
            "Failed to load ItemRandomProperties.db2 — check DataDir and DBC.Locale config",
        )?,
    );
    info!(
        "Loaded {} item random properties from ItemRandomProperties.db2",
        item_random_properties_store.len()
    );

    // Load ItemSpecOverride.db2 for C++ ObjectMgr::LoadItemTemplates ItemSpecClassMask primary path.
    let item_spec_override_store = Arc::new(
        wow_data::ItemSpecOverrideStore::load(&data_dir, &locale)
            .context("Failed to load ItemSpecOverride.db2 — check DataDir and DBC.Locale config")?,
    );
    info!(
        "Loaded {} item spec overrides from ItemSpecOverride.db2",
        item_spec_override_store.len()
    );

    let rand_prop_points_store = Arc::new(
        wow_data::RandPropPointsStore::load(&data_dir, &locale)
            .context("Failed to load RandPropPoints.db2 — check DataDir and DBC.Locale config")?,
    );
    info!(
        "Loaded {} random property point rows from RandPropPoints.db2",
        rand_prop_points_store.len()
    );

    // Load ItemDisenchantLoot.db2 for C++ sItemDisenchantLootStore lookup.
    let item_disenchant_loot_store = Arc::new(
        wow_data::ItemDisenchantLootStore::load(&data_dir, &locale).context(
            "Failed to load ItemDisenchantLoot.db2 — check DataDir and DBC.Locale config",
        )?,
    );
    info!(
        "Loaded {} item disenchant loot rows from ItemDisenchantLoot.db2",
        item_disenchant_loot_store.len()
    );

    let item_random_enchantment_persistence =
        wow_database::MariaDbItemRandomEnchantmentCatalogPersistenceAdapterLikeCpp::new(
            Arc::clone(&world_db),
        );
    let item_random_enchantment_template_store = Arc::new(
        crate::item_random_enchantment_catalog::load_item_random_enchantment_store_like_cpp(
            &item_random_enchantment_persistence,
            &item_random_properties_store,
            &item_random_suffix_store,
        )
        .await
        .context("Failed to load item_random_enchantment_template")?,
    );

    // Load SpellItemEnchantment.db2 for ApplyEnchantment and arena enchantment checks.
    let spell_item_enchantment_store = Arc::new(
        wow_data::SpellItemEnchantmentStore::load(&data_dir, &locale).context(
            "Failed to load SpellItemEnchantment.db2 — check DataDir and DBC.Locale config",
        )?,
    );
    info!(
        "Loaded {} spell item enchantments from SpellItemEnchantment.db2",
        spell_item_enchantment_store.len()
    );
    let spell_item_enchantment_condition_store = Arc::new(
        wow_data::SpellItemEnchantmentConditionStore::load(&data_dir, &locale).context(
            "Failed to load SpellItemEnchantmentCondition.db2 — check DataDir and DBC.Locale config",
        )?,
    );
    info!(
        "Loaded {} spell item enchantment conditions from SpellItemEnchantmentCondition.db2",
        spell_item_enchantment_condition_store.len()
    );
    let gem_properties_store = Arc::new(
        wow_data::GemPropertiesStore::load(&data_dir, &locale)
            .context("Failed to load GemProperties.db2 — check DataDir and DBC.Locale config")?,
    );
    info!(
        "Loaded {} gem properties from GemProperties.db2",
        gem_properties_store.len()
    );
    let spell_enchant_proc_outcome =
        crate::static_data_overlay::load_spell_enchant_proc_store_like_cpp(
            &static_data_overlay_persistence,
            spell_item_enchantment_store.as_ref(),
        )
        .await
        .context("Failed to load C++ spell_enchant_proc_data rows")?;
    info!(
        "Loaded {} C++ spell_enchant_proc_data rows ({} missing enchantments)",
        spell_enchant_proc_outcome.loaded_row_count,
        spell_enchant_proc_outcome.errors.len()
    );

    // Build hotfix blob cache — pre-loads raw DB2 record bytes and hotfix DB overlays for DBReply.
    let mut hotfix_blob_cache = wow_data::build_hotfix_blob_cache(&data_dir, &locale);
    let [hotfix_blobs, hotfix_data, hotfix_optional_data] =
        crate::hotfix_delivery_metadata::load_hotfix_delivery_metadata_like_cpp(
            &mut hotfix_blob_cache,
            &hotfix_delivery_metadata_persistence,
            &locale,
        )
        .await;
    match hotfix_blobs {
        Ok(n) => info!("HotfixBlobCache: loaded {n} hotfix_blob rows"),
        Err(e) => tracing::warn!("HotfixBlobCache: failed to load hotfix_blob rows: {e}"),
    }
    match hotfix_data {
        Ok(n) => info!("HotfixBlobCache: loaded {n} hotfix_data rows"),
        Err(e) => tracing::warn!("HotfixBlobCache: failed to load hotfix_data rows: {e}"),
    }
    match hotfix_optional_data {
        Ok(n) => info!("HotfixBlobCache: loaded {n} hotfix_optional_data rows"),
        Err(e) => tracing::warn!("HotfixBlobCache: failed to load hotfix_optional_data rows: {e}"),
    }
    let hotfix_blob_cache = Arc::new(hotfix_blob_cache);
    let tact_key_store = Arc::new(
        wow_data::TactKeyStore::load(&data_dir, &locale).context("Failed to load TactKey.db2")?,
    );
    info!(
        "Loaded {} TactKey rows from TactKey.db2",
        tact_key_store.len()
    );

    // Load spell metadata (cast time, cooldown, effects, etc.) — Phase 2
    let spell_radius_store = Arc::new(
        spell_core_db2_hotfix::load_spell_radius_store_like_cpp(
            &data_dir,
            &locale,
            &spell_core_hotfix_persistence,
            &db2_hotfix_removals,
        )
        .await
        .context("Failed to load effective SpellRadius authority")?,
    );
    info!("Loaded {} spell radius rows", spell_radius_store.len());
    let spell_range_store = Arc::new(
        spell_core_db2_hotfix::load_spell_range_store_like_cpp(
            &data_dir,
            &locale,
            &spell_core_hotfix_persistence,
            &db2_hotfix_removals,
        )
        .await
        .context("Failed to load effective SpellRange authority")?,
    );
    info!("Loaded {} spell range rows", spell_range_store.len());
    let serverside_spell_effect_outcome =
        spell_acquisition_loader::load_serverside_spell_effects_like_cpp(
            &spell_acquisition_startup_persistence,
            |spell_id| spell_store.contains_spell_info_any_difficulty_like_cpp(spell_id),
            |difficulty_id| difficulty_store.get(difficulty_id).is_some(),
            |radius_id| spell_radius_store.get(radius_id).is_some(),
        )
        .await
        .context("Failed to load C++ serverside_spell_effect rows")?;
    let serverside_spell_effect_store = serverside_spell_effect_outcome.store;
    info!(
        "Loaded {} C++ serverside_spell_effect rows ({} validation errors; {} radius warnings)",
        serverside_spell_effect_outcome.loaded_effect_count,
        serverside_spell_effect_outcome.errors.len(),
        serverside_spell_effect_outcome.warnings.len()
    );
    let serverside_spell_outcome = spell_acquisition_loader::load_serverside_spells_like_cpp(
        &spell_acquisition_startup_persistence,
        &serverside_spell_effect_store,
        |spell_id| spell_name_store.get(spell_id).is_some(),
    )
    .await
    .context("Failed to load C++ serverside_spell rows")?;
    spell_store.apply_serverside_spell_interrupts_like_cpp(&serverside_spell_outcome.store);
    let serverside_spell_store = Arc::new(serverside_spell_outcome.store);
    info!(
        "Loaded {} C++ serverside_spell rows ({} validation errors; authoritative SpellInfo insertion still pending)",
        serverside_spell_outcome.loaded_spell_count,
        serverside_spell_outcome.errors.len()
    );

    let spell_acquisition_bootstrap = spell_acquisition_loader::load_like_cpp(
        &data_dir,
        &locale,
        &spell_acquisition_startup_persistence,
        &db2_hotfix_removals,
        &spell_store,
        serverside_spell_store.as_ref(),
        difficulty_store.as_ref(),
        skill_store.as_ref(),
    )
    .await
    .context("Failed to compose effective spell-acquisition stores")?;
    let spell_acquisition_catalog = spell_acquisition_bootstrap.catalog;
    let spell_chain_store = spell_acquisition_bootstrap.chain_store;
    let spell_learn_skill_store = spell_acquisition_bootstrap.learn_skill_store;
    let spell_learn_spell_store = spell_acquisition_bootstrap.learn_spell_store;
    let spell_custom_attribute_store = spell_acquisition_bootstrap.custom_attribute_store;

    // Load area trigger store (collision detection + teleportation)
    let area_trigger_world_persistence =
        wow_database::MariaDbAreaTriggerWorldCatalogPersistenceAdapterLikeCpp::new(Arc::clone(
            &world_db,
        ));
    let area_trigger_world_catalogs =
        crate::area_trigger_world_catalog::load_area_trigger_world_catalogs_like_cpp(
            &area_trigger_world_persistence,
            area_trigger_db2_store.as_ref(),
            Arc::make_mut(&mut script_name_interner),
        )
        .await?;
    let area_trigger_store = area_trigger_world_catalogs.area_trigger_store;
    let area_trigger_script_outcome = area_trigger_world_catalogs.script_outcome;
    let area_trigger_script_store = Arc::new(area_trigger_script_outcome.store);
    info!(
        "Loaded {} C++ area trigger script bindings ({} skipped missing area trigger)",
        area_trigger_script_store.len(),
        area_trigger_script_outcome
            .report
            .skipped_missing_area_trigger
            .len()
    );
    let tavern_area_trigger_outcome = area_trigger_world_catalogs.tavern_outcome;
    let tavern_area_trigger_store = Arc::new(tavern_area_trigger_outcome.store);
    info!(
        "Loaded {} C++ tavern area triggers ({} rows seen; {} skipped missing AreaTrigger.db2)",
        tavern_area_trigger_store.len(),
        tavern_area_trigger_outcome.report.rows_seen,
        tavern_area_trigger_outcome
            .report
            .skipped_missing_area_trigger
            .len()
    );

    // Load quest store (templates + objectives + NPC relations)
    let quest_store = Arc::new(
        crate::quest_catalog::load_quests_like_cpp(&quest_catalog_persistence)
            .await
            .context("Failed to load quest store")?,
    );
    let lfg_world_catalog_persistence =
        wow_database::MariaDbLfgWorldCatalogPersistenceAdapterLikeCpp::new(Arc::clone(&world_db));
    let lfg_load_outcome = lfg_world_catalog::load_lfg_dungeon_store_like_cpp(
        &lfg_world_catalog_persistence,
        lfg_dungeons_store.as_ref(),
        map_difficulty_store.as_ref(),
        quest_store.as_ref(),
    )
    .await
    .context("Failed to load C++ LFG dungeon store")?;
    let lfg_dungeon_store_like_cpp = Arc::new(lfg_load_outcome.store);
    info!(
        "Loaded {} C++ LFG dungeon rows ({} templates, {} rewards; {} skipped db2 type, {} skipped map difficulty)",
        lfg_dungeon_store_like_cpp.len(),
        lfg_load_outcome.report.loaded_templates,
        lfg_load_outcome.report.loaded_rewards,
        lfg_load_outcome.report.skipped_type.len(),
        lfg_load_outcome.report.skipped_missing_map_difficulty.len(),
    );
    if std::env::var_os("RUSTYCORE_LFG_TRACE").is_some() {
        for id in [
            205_u32, 210, 211, 212, 213, 215, 217, 219, 221, 226, 241, 242, 245, 249, 252, 253,
            254, 255, 256, 259, 260, 2447, 2452, 2471,
        ] {
            match lfg_dungeon_store_like_cpp.get(id) {
                Some(dungeon) => info!(
                    id,
                    entry = dungeon.entry_like_cpp(),
                    type_id = dungeon.type_id,
                    map = dungeon.map,
                    difficulty = dungeon.difficulty,
                    expansion = dungeon.expansion,
                    group = dungeon.group,
                    min_level = dungeon.min_level,
                    max_level = dungeon.max_level,
                    required_item_level = dungeon.required_item_level,
                    seasonal = dungeon.seasonal,
                    "RUST_LFG_TRACE dungeon"
                ),
                None => info!(id, "RUST_LFG_TRACE dungeon missing"),
            }
        }
        let random_ids = lfg_dungeon_store_like_cpp
            .random_and_active_seasonal_dungeon_entries_like_cpp(80, 2, |_| false);
        info!(
            ?random_ids,
            "RUST_LFG_TRACE random entries level80 expansion2"
        );
    }
    let spell_world_catalog_persistence =
        wow_database::MariaDbSpellWorldCatalogPersistenceAdapterLikeCpp::new(Arc::clone(&world_db));
    let spell_area_outcome = crate::spell_world_catalog::load_spell_area_like_cpp(
        &spell_world_catalog_persistence,
        |spell_id| spell_store.get(spell_id as i32).is_some(),
        |area_id| area_table_store.get(area_id).is_some(),
        |quest_id| quest_store.get(quest_id).is_some(),
    )
    .await
    .context("Failed to load C++ spell_area rows")?;
    let spell_area_store = Arc::new(spell_area_outcome.store);
    info!(
        "Loaded {} C++ spell_area rows ({} validation issues; SpellInfo no-aura-cancel mutation still pending)",
        spell_area_outcome.loaded_row_count,
        spell_area_outcome.errors.len()
    );
    let access_requirement_outcome =
        crate::world_auxiliary_catalog::load_access_requirements_like_cpp(
            &world_auxiliary_catalog_persistence,
            &map_store,
            &map_difficulty_store,
            &item_store,
            quest_store.as_ref(),
            achievement_store.as_ref(),
        )
        .await
        .context("Failed to load C++ access_requirement rows")?;
    let access_requirement_store = Arc::new(access_requirement_outcome.store);
    info!(
        "Loaded {} C++ access requirement rows ({} rows seen; {} map/difficulty skips; {} reference clears)",
        access_requirement_outcome.report.loaded_rows,
        access_requirement_outcome.report.rows_seen,
        access_requirement_outcome.report.skipped_missing_map.len()
            + access_requirement_outcome
                .report
                .skipped_missing_difficulty
                .len(),
        access_requirement_outcome.report.cleared_missing_item.len()
            + access_requirement_outcome
                .report
                .cleared_missing_item2
                .len()
            + access_requirement_outcome
                .report
                .cleared_missing_quest_a
                .len()
            + access_requirement_outcome
                .report
                .cleared_missing_quest_h
                .len()
            + access_requirement_outcome
                .report
                .cleared_missing_achievement
                .len()
    );
    let disable_mgr = Arc::new(
        load_disable_mgr_like_cpp(
            &condition_disable_catalog_persistence,
            &map_store,
            &map_difficulty_store,
            &spell_store,
            quest_store.as_ref(),
            criteria_store.as_ref(),
            battlemaster_list_store.as_ref(),
        )
        .await?,
    );
    let mmap_disabled_map_ids = disable_mgr.disabled_mmap_map_ids_like_cpp();
    info!(
        "Loaded {} C++ mmap disable rows",
        mmap_disabled_map_ids.len()
    );

    let loaded_loot_stores = load_loot_stores_like_cpp(&world_db, &item_store)
        .await
        .context("Failed to load C++ LootTemplates_* foundation stores")?;
    let loot_reference_report = check_loot_references_like_cpp(&loaded_loot_stores);
    log_loot_reference_report_like_cpp(&loot_reference_report);
    let loot_condition_ids = load_loot_condition_ids_like_cpp(&world_db)
        .await
        .context("Failed to load C++ loot-template condition IDs")?;
    let mut loot_condition_report =
        check_loot_condition_links_like_cpp(&loaded_loot_stores, loot_condition_ids, |item_id| {
            item_store.get(item_id).is_some()
        });
    let loot_condition_reference_uses = load_loot_condition_reference_uses_like_cpp(&world_db)
        .await
        .context("Failed to load C++ loot-template condition reference uses")?;
    let condition_reference_template_ids =
        load_condition_reference_template_ids_like_cpp(&world_db)
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
    let gameobject_for_quest_store = Arc::new(
        wow_data::GameObjectForQuestStoreLikeCpp::from_templates_like_cpp(
            gameobject_template_lifecycle_store.as_ref(),
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
        gameobject_for_quest_store.len()
    );
    let reserved_name_persistence =
        wow_database::MariaDbReservedNameCatalogPersistenceAdapterLikeCpp::new(Arc::clone(
            &char_db,
        ));
    let reserved_name_store = Arc::new(
        crate::reserved_name_catalog::load_reserved_name_catalog_like_cpp(
            &reserved_name_persistence,
        )
        .await
        .context("Failed to load C++ reserved player names")?,
    );
    info!(
        "Loaded {} C++ reserved player names ({} unique)",
        reserved_name_store.loaded_rows_like_cpp(),
        reserved_name_store.len()
    );
    let game_tele_persistence =
        wow_database::MariaDbGameTeleCatalogPersistenceAdapterLikeCpp::new(Arc::clone(&world_db));
    let game_tele_outcome =
        crate::game_tele_catalog::load_game_tele_catalog_like_cpp(&game_tele_persistence)
            .await
            .context("Failed to load C++ game teleport locations")?;
    for (id, name) in &game_tele_outcome.report.skipped_invalid_coordinates {
        tracing::error!(
            "Wrong position for id {} (name: {}) in `game_tele` table, ignoring.",
            id,
            name
        );
    }
    let game_tele_store = Arc::new(game_tele_outcome.store);
    info!(
        "Loaded {} C++ GameTeleports ({} unique ids)",
        game_tele_outcome.report.loaded_rows,
        game_tele_store.len()
    );
    let npc_vendor_outcome = crate::gameplay_rule_catalog::load_npc_vendor_store_like_cpp(
        &gameplay_rule_catalog_persistence,
    )
    .await
    .context("Failed to load C++ NPC vendor item cache")?;
    for (entry, item) in &npc_vendor_outcome
        .report
        .skipped_item_maxcount_without_incrtime
    {
        tracing::error!(
            "Table `(game_event_)npc_vendor` has `maxcount` set for item {} of vendor (Entry: {}) but `incrtime`=0, ignoring",
            item,
            entry
        );
    }
    for (entry, item) in &npc_vendor_outcome
        .report
        .skipped_item_incrtime_without_maxcount
    {
        tracing::error!(
            "Table `(game_event_)npc_vendor` has `maxcount`=0 for item {} of vendor (Entry: {}) but `incrtime`<>0, ignoring",
            item,
            entry
        );
    }
    for (entry, item) in &npc_vendor_outcome.report.skipped_currency_without_maxcount {
        tracing::error!(
            "Table `(game_event_)npc_vendor` has currency item {} with missing maxcount for vendor ({}), ignoring",
            item,
            entry
        );
    }
    for (entry, item, extended_cost, vendor_type) in &npc_vendor_outcome.report.skipped_duplicates {
        tracing::error!(
            "Table `npc_vendor` has duplicate items {} (with extended cost {}, type {}) for vendor (Entry: {}), ignoring",
            item,
            extended_cost,
            vendor_type,
            entry
        );
    }
    for (entry, reference_entry) in &npc_vendor_outcome.report.skipped_reference_cycles {
        tracing::error!(
            "Table `npc_vendor` has cyclic reference vendor {} while loading vendor {}, ignoring nested reference",
            reference_entry,
            entry
        );
    }
    let npc_vendor_store = Arc::new(npc_vendor_outcome.store);
    info!(
        "Loaded {} C++ vendor items across {} NPC vendors ({} reference rows expanded)",
        npc_vendor_outcome.report.loaded_items,
        npc_vendor_store.len(),
        npc_vendor_outcome.report.reference_rows_seen
    );
    let trainer_catalog_persistence =
        wow_database::MariaDbTrainerCatalogPersistenceAdapterLikeCpp::new(Arc::clone(&world_db));
    let trainer_data_outcome = crate::trainer_catalog::load_trainer_catalog_like_cpp(
        &trainer_catalog_persistence,
        |spell_id| {
            spell_store.contains_spell_info_difficulty_none_like_cpp(
                serverside_spell_store.as_ref(),
                difficulty_store.as_ref(),
                spell_id,
            )
        },
        |skill_line_id| skill_line_store.contains_effective_record_like_cpp(skill_line_id),
        |creature_id| creature_template_store.contains(creature_id),
        |menu_id, option_id| {
            gossip_store
                .menu_items_for_id(menu_id)
                .is_some_and(|items| items.iter().any(|item| item.order_index == option_id))
        },
    )
    .await
    .context("Failed to load C++ trainer cache")?;
    for diagnostic in &trainer_data_outcome
        .report
        .diagnostics_in_load_order_like_cpp
    {
        match diagnostic {
            wow_data::TrainerLoadDiagnosticLikeCpp::TrainerSpellMissingSpell {
                trainer_id,
                spell_id,
            } => tracing::error!(
                "Table `trainer_spell` references non-existing spell (SpellId: {}) for TrainerId {}, ignoring",
                spell_id,
                trainer_id
            ),
            wow_data::TrainerLoadDiagnosticLikeCpp::TrainerSpellMissingSkillLine {
                trainer_id,
                spell_id,
                skill_line_id,
            } => tracing::error!(
                "Table `trainer_spell` references non-existing skill (ReqSkillLine: {}) for TrainerId {} and SpellId {}, ignoring",
                skill_line_id,
                trainer_id,
                spell_id
            ),
            wow_data::TrainerLoadDiagnosticLikeCpp::TrainerSpellMissingRequiredSpell {
                trainer_id,
                spell_id,
                required_index,
                required_spell_id,
            } => tracing::error!(
                "Table `trainer_spell` references non-existing spell (ReqAbility{}: {}) for TrainerId {} and SpellId {}, ignoring",
                required_index,
                required_spell_id,
                trainer_id,
                spell_id
            ),
            wow_data::TrainerLoadDiagnosticLikeCpp::TrainerSpellMissingTrainer {
                trainer_id,
                spell_id,
            } => tracing::error!(
                "Table `trainer_spell` references non-existing trainer (TrainerId: {}) for SpellId {}, ignoring",
                trainer_id,
                spell_id
            ),
            wow_data::TrainerLoadDiagnosticLikeCpp::TrainerLocaleMissingTrainer {
                trainer_id,
                locale,
            } => tracing::error!(
                "Table `trainer_locale` references non-existing trainer (TrainerId: {}) for locale {}, ignoring",
                trainer_id,
                locale
            ),
            wow_data::TrainerLoadDiagnosticLikeCpp::CreatureTrainerMissingCreatureTemplate {
                creature_id,
            } => tracing::error!(
                "Table `creature_trainer` references non-existing creature template (CreatureID: {}), ignoring",
                creature_id
            ),
            wow_data::TrainerLoadDiagnosticLikeCpp::CreatureTrainerMissingTrainer {
                creature_id,
                trainer_id,
                menu_id,
                option_id,
            } => tracing::error!(
                "Table `creature_trainer` references non-existing trainer (TrainerID: {}) for CreatureID {} MenuID {} OptionID {}, ignoring",
                trainer_id,
                creature_id,
                menu_id,
                option_id
            ),
            wow_data::TrainerLoadDiagnosticLikeCpp::CreatureTrainerMissingGossipOption {
                creature_id,
                trainer_id,
                menu_id,
                option_id,
            } => tracing::error!(
                "Table `creature_trainer` references non-existing gossip menu option (MenuID {} OptionID {}) for CreatureID {} and TrainerID {}, ignoring",
                menu_id,
                option_id,
                creature_id,
                trainer_id
            ),
        }
    }
    let trainer_data_store = Arc::new(trainer_data_outcome.store);
    info!(
        "Loaded {} C++ Trainers with {} trainer spells and {} creature trainer bindings",
        trainer_data_store.len(),
        trainer_data_store.spell_count_like_cpp(),
        trainer_data_store.creature_trainer_count_like_cpp()
    );

    // C++ loads breeds and qualities independently and tolerates either
    // unavailable table as an empty catalog.
    let battle_pet_selection_persistence =
        wow_database::MariaDbBattlePetSelectionCatalogPersistenceAdapterLikeCpp::new(Arc::clone(
            &world_db,
        ));
    let battle_pet_selection_store = Arc::new(
        crate::battle_pet_selection_catalog::load_battle_pet_selection_store_like_cpp(
            &battle_pet_selection_persistence,
            |species| {
                battle_pet_species_entry_store
                    .get(species)
                    .map(|entry| entry.flags)
            },
        )
        .await,
    );
    info!(
        "Loaded {} battle-pet breed/quality selection rows",
        battle_pet_selection_store.len_like_cpp()
    );

    let mut faction_change_outcome =
        crate::gameplay_rule_catalog::load_faction_change_store_like_cpp(
            &gameplay_rule_catalog_persistence,
            |id| achievement_store.contains(id),
            |id| quest_store.get(id).is_some(),
            |id| faction_store.contains(id),
            |id| spell_store.get(i32::try_from(id).unwrap_or(-1)).is_some(),
            |id| char_titles_store.contains(id),
        )
        .await
        .context("Failed to load C++ faction-change mapping stores")?;
    faction_change_outcome.store = faction_change_outcome.store.with_item_templates_like_cpp(
        item_stats_store
            .sparse_templates_like_cpp()
            .map(
                |(item_id, template)| wow_data::FactionChangeItemTemplateLikeCpp {
                    item_id,
                    other_faction_item_id: template.other_faction_item_id_like_cpp(),
                    flags2: template.flags[1],
                },
            ),
        &mut faction_change_outcome.report,
    );
    for error in &faction_change_outcome.report.validation_errors {
        tracing::error!("{}", error.cpp_message_like_cpp());
    }
    info!(
        "Loaded C++ faction-change pairs: achievements {} rows/{} valid, spells {} rows/{} valid, quests {} rows/{} valid, items {} derived ({} Alliance->Horde, {} Horde->Alliance), reputations {} rows/{} valid, titles {} rows/{} valid ({} validation issues)",
        faction_change_outcome.report.achievement_rows_seen,
        faction_change_outcome.store.achievement_len(),
        faction_change_outcome.report.spell_rows_seen,
        faction_change_outcome.store.spell_len(),
        faction_change_outcome.report.quest_rows_seen,
        faction_change_outcome.store.quest_len(),
        faction_change_outcome.report.item_rows_seen,
        faction_change_outcome.store.item_alliance_to_horde_len(),
        faction_change_outcome.store.item_horde_to_alliance_len(),
        faction_change_outcome.report.reputation_rows_seen,
        faction_change_outcome.store.reputation_len(),
        faction_change_outcome.report.title_rows_seen,
        faction_change_outcome.store.title_len(),
        faction_change_outcome.report.validation_errors.len()
    );
    let _faction_change_store = Arc::new(faction_change_outcome.store);

    // Load player_xp_for_level table
    let player_xp_table = {
        let stmt = world_db.prepare(WorldStatements::SEL_PLAYER_XP_FOR_LEVEL);
        let mut table = vec![0u32; 82]; // index = level, 0=unused, 81=max
        if let Ok(result) = world_db.query(&stmt).await {
            let mut r = result;
            loop {
                let lvl: u8 = r.try_read::<u8>(0).unwrap_or(0);
                let xp: u32 = r
                    .try_read::<u32>(1)
                    .or_else(|| r.try_read::<i32>(1).map(|value| value as u32))
                    .unwrap_or(0);
                if (lvl as usize) < table.len() {
                    table[lvl as usize] = xp;
                }
                if !r.next_row() {
                    break;
                }
            }
        }
        Arc::new(table)
    };
    let exploration_base_xp_persistence =
        wow_database::MariaDbExplorationBaseXpCatalogPersistenceAdapterLikeCpp::new(Arc::clone(
            &world_db,
        ));
    let exploration_base_xp_store = Arc::new(
        crate::exploration_base_xp_catalog::load_exploration_base_xp_catalog_like_cpp(
            &exploration_base_xp_persistence,
        )
        .await?,
    );

    // Load QuestXP.db2 for accurate XP rewards
    let dbc_path = format!("{}/dbc/{}", data_dir, locale);
    let quest_xp_store = Arc::new(
        wow_data::quest_xp::QuestXpStore::load(&dbc_path).unwrap_or_else(|e| {
            tracing::warn!("QuestXP.db2 not loaded ({e}), using fallback XP table");
            wow_data::quest_xp::QuestXpStore::default()
        }),
    );
    let quest_money_reward_store = Arc::new(
        wow_data::progression_rewards::QuestMoneyRewardStore::load(&data_dir, &locale)
            .context("Failed to load QuestMoneyReward.db2 — check DataDir and DBC.Locale config")?,
    );
    let quest_v2_store = Arc::new(
        wow_data::progression_rewards::QuestV2Store::load(&data_dir, &locale)
            .context("Failed to load QuestV2.db2 — check DataDir and DBC.Locale config")?,
    );
    let quest_info_store = Arc::new(
        wow_data::progression_rewards::QuestInfoStore::load(&data_dir, &locale)
            .context("Failed to load QuestInfo.db2 — check DataDir and DBC.Locale config")?,
    );
    let quest_package_item_store = Arc::new(
        wow_data::progression_rewards::QuestPackageItemStore::load(&data_dir, &locale)
            .context("Failed to load QuestPackageItem.db2 — check DataDir and DBC.Locale config")?,
    );
    let player_choice_catalog_persistence =
        wow_database::MariaDbPlayerChoiceCatalogPersistenceAdapterLikeCpp::new(Arc::clone(
            &world_db,
        ));
    let mut player_choice_outcome = crate::player_choice_catalog::load_core_like_cpp(
        &player_choice_catalog_persistence,
        |title_id| char_titles_store.contains(title_id),
        |package_id| {
            quest_package_item_store
                .quest_package_items_like_cpp(package_id)
                .next()
                .is_some()
                || quest_package_item_store
                    .quest_package_items_fallback_like_cpp(package_id)
                    .next()
                    .is_some()
        },
        |skill_line_id| {
            skill_line_store.contains_effective_record_like_cpp(skill_line_id)
        },
        |item_id| item_stats_store.sparse_template(item_id).is_some(),
        |currency_id| currency_types_store.has_record(currency_id),
        |faction_id| faction_store.contains(faction_id),
    )
    .await
    .context(
        "Failed to load C++ playerchoice/playerchoice_response/playerchoice_response_reward/playerchoice_response_reward_item/playerchoice_response_reward_currency/playerchoice_response_reward_faction/playerchoice_response_reward_item_choice/playerchoice_response_maw_power rows",
    )?;
    for (choice_id, response_id) in &player_choice_outcome
        .report
        .skipped_responses_missing_choice
    {
        tracing::error!(
            target: "sql.sql",
            "Table `playerchoice_response` references non-existing ChoiceId: {} (ResponseId: {}), skipped",
            choice_id,
            response_id
        );
    }
    for (choice_id, response_id) in &player_choice_outcome.report.skipped_rewards_missing_choice {
        tracing::error!(
            target: "sql.sql",
            "Table `playerchoice_response_reward` references non-existing ChoiceId: {} (ResponseId: {}), skipped",
            choice_id,
            response_id
        );
    }
    for (choice_id, response_id) in &player_choice_outcome
        .report
        .skipped_rewards_missing_response
    {
        tracing::error!(
            target: "sql.sql",
            "Table `playerchoice_response_reward` references non-existing ResponseId: {} for ChoiceId {}, skipped",
            response_id,
            choice_id
        );
    }
    for (choice_id, response_id, title_id) in &player_choice_outcome.report.invalid_reward_titles {
        tracing::error!(
            target: "sql.sql",
            "Table `playerchoice_response_reward` references non-existing Title {} for ChoiceId {}, ResponseId: {}, set to 0",
            title_id,
            choice_id,
            response_id
        );
    }
    for (choice_id, response_id, package_id) in
        &player_choice_outcome.report.invalid_reward_packages
    {
        tracing::error!(
            target: "sql.sql",
            "Table `playerchoice_response_reward` references non-existing QuestPackage {} for ChoiceId {}, ResponseId: {}, set to 0",
            package_id,
            choice_id,
            response_id
        );
    }
    for (choice_id, response_id, skill_line_id) in
        &player_choice_outcome.report.invalid_reward_skill_lines
    {
        tracing::error!(
            target: "sql.sql",
            "Table `playerchoice_response_reward` references non-existing SkillLine {} for ChoiceId {}, ResponseId: {}, set to 0",
            skill_line_id,
            choice_id,
            response_id
        );
    }
    for (choice_id, response_id) in &player_choice_outcome
        .report
        .skipped_reward_items_missing_choice
    {
        tracing::error!(
            target: "sql.sql",
            "Table `playerchoice_response_reward_item` references non-existing ChoiceId: {} (ResponseId: {}), skipped",
            choice_id,
            response_id
        );
    }
    for (choice_id, response_id) in &player_choice_outcome
        .report
        .skipped_reward_items_missing_response
    {
        tracing::error!(
            target: "sql.sql",
            "Table `playerchoice_response_reward_item` references non-existing ResponseId: {} for ChoiceId {}, skipped",
            response_id,
            choice_id
        );
    }
    for (choice_id, response_id) in &player_choice_outcome
        .report
        .skipped_reward_items_missing_reward
    {
        tracing::error!(
            target: "sql.sql",
            "Table `playerchoice_response_reward_item` references non-existing player choice reward for ChoiceId {}, ResponseId: {}, skipped",
            choice_id,
            response_id
        );
    }
    for (choice_id, response_id, item_id) in &player_choice_outcome
        .report
        .skipped_reward_items_missing_item
    {
        tracing::error!(
            target: "sql.sql",
            "Table `playerchoice_response_reward_item` references non-existing item {} for ChoiceId {}, ResponseId: {}, skipped",
            item_id,
            choice_id,
            response_id
        );
    }
    for (choice_id, response_id) in &player_choice_outcome
        .report
        .skipped_reward_currencies_missing_choice
    {
        tracing::error!(
            target: "sql.sql",
            "Table `playerchoice_response_reward_currency` references non-existing ChoiceId: {} (ResponseId: {}), skipped",
            choice_id,
            response_id
        );
    }
    for (choice_id, response_id) in &player_choice_outcome
        .report
        .skipped_reward_currencies_missing_response
    {
        tracing::error!(
            target: "sql.sql",
            "Table `playerchoice_response_reward_currency` references non-existing ResponseId: {} for ChoiceId {}, skipped",
            response_id,
            choice_id
        );
    }
    for (choice_id, response_id) in &player_choice_outcome
        .report
        .skipped_reward_currencies_missing_reward
    {
        tracing::error!(
            target: "sql.sql",
            "Table `playerchoice_response_reward_currency` references non-existing player choice reward for ChoiceId {}, ResponseId: {}, skipped",
            choice_id,
            response_id
        );
    }
    for (choice_id, response_id, currency_id) in &player_choice_outcome
        .report
        .skipped_reward_currencies_missing_currency
    {
        tracing::error!(
            target: "sql.sql",
            "Table `playerchoice_response_reward_currency` references non-existing currency {} for ChoiceId {}, ResponseId: {}, skipped",
            currency_id,
            choice_id,
            response_id
        );
    }
    for (choice_id, response_id) in &player_choice_outcome
        .report
        .skipped_reward_factions_missing_choice
    {
        tracing::error!(
            target: "sql.sql",
            "Table `playerchoice_response_reward_faction` references non-existing ChoiceId: {} (ResponseId: {}), skipped",
            choice_id,
            response_id
        );
    }
    for (choice_id, response_id) in &player_choice_outcome
        .report
        .skipped_reward_factions_missing_response
    {
        tracing::error!(
            target: "sql.sql",
            "Table `playerchoice_response_reward_faction` references non-existing ResponseId: {} for ChoiceId {}, skipped",
            response_id,
            choice_id
        );
    }
    for (choice_id, response_id) in &player_choice_outcome
        .report
        .skipped_reward_factions_missing_reward
    {
        tracing::error!(
            target: "sql.sql",
            "Table `playerchoice_response_reward_faction` references non-existing player choice reward for ChoiceId {}, ResponseId: {}, skipped",
            choice_id,
            response_id
        );
    }
    for (choice_id, response_id, faction_id) in &player_choice_outcome
        .report
        .skipped_reward_factions_missing_faction
    {
        tracing::error!(
            target: "sql.sql",
            "Table `playerchoice_response_reward_faction` references non-existing faction {} for ChoiceId {}, ResponseId: {}, skipped",
            faction_id,
            choice_id,
            response_id
        );
    }
    for (choice_id, response_id) in &player_choice_outcome
        .report
        .skipped_reward_item_choices_missing_choice
    {
        tracing::error!(
            target: "sql.sql",
            "Table `playerchoice_response_reward_item_choice` references non-existing ChoiceId: {} (ResponseId: {}), skipped",
            choice_id,
            response_id
        );
    }
    for (choice_id, response_id) in &player_choice_outcome
        .report
        .skipped_reward_item_choices_missing_response
    {
        tracing::error!(
            target: "sql.sql",
            "Table `playerchoice_response_reward_item_choice` references non-existing ResponseId: {} for ChoiceId {}, skipped",
            response_id,
            choice_id
        );
    }
    for (choice_id, response_id) in &player_choice_outcome
        .report
        .skipped_reward_item_choices_missing_reward
    {
        tracing::error!(
            target: "sql.sql",
            "Table `playerchoice_response_reward_item_choice` references non-existing player choice reward for ChoiceId {}, ResponseId: {}, skipped",
            choice_id,
            response_id
        );
    }
    for (choice_id, response_id, item_id) in &player_choice_outcome
        .report
        .skipped_reward_item_choices_missing_item
    {
        tracing::error!(
            target: "sql.sql",
            "Table `playerchoice_response_reward_item_choice` references non-existing item {} for ChoiceId {}, ResponseId: {}, skipped",
            item_id,
            choice_id,
            response_id
        );
    }
    for (choice_id, response_id) in &player_choice_outcome
        .report
        .skipped_maw_powers_missing_choice
    {
        tracing::error!(
            target: "sql.sql",
            "Table `playerchoice_response_maw_power` references non-existing ChoiceId: {} (ResponseId: {}), skipped",
            choice_id,
            response_id
        );
    }
    for (choice_id, response_id) in &player_choice_outcome
        .report
        .skipped_maw_powers_missing_response
    {
        tracing::error!(
            target: "sql.sql",
            "Table `playerchoice_response_maw_power` references non-existing ResponseId: {} for ChoiceId {}, skipped",
            response_id,
            choice_id
        );
    }
    info!(
        "Loaded {} C++ player choices with {} responses, {} base rewards, {} reward items, {} reward currencies, {} reward factions, {} reward item choices, and {} maw powers ({} skipped responses, {} skipped rewards, {} skipped reward items, {} skipped reward currencies, {} skipped reward factions, {} skipped reward item choices, {} skipped maw powers, {} invalid reward refs; live DisplayPlayerChoice flow pending)",
        player_choice_outcome.report.choice_rows_seen,
        player_choice_outcome.report.loaded_responses,
        player_choice_outcome.report.loaded_rewards,
        player_choice_outcome.report.loaded_reward_items,
        player_choice_outcome.report.loaded_reward_currencies,
        player_choice_outcome.report.loaded_reward_factions,
        player_choice_outcome.report.loaded_reward_item_choices,
        player_choice_outcome.report.loaded_maw_powers,
        player_choice_outcome
            .report
            .skipped_responses_missing_choice
            .len(),
        player_choice_outcome
            .report
            .skipped_rewards_missing_choice
            .len()
            + player_choice_outcome
                .report
                .skipped_rewards_missing_response
                .len(),
        player_choice_outcome
            .report
            .skipped_reward_items_missing_choice
            .len()
            + player_choice_outcome
                .report
                .skipped_reward_items_missing_response
                .len()
            + player_choice_outcome
                .report
                .skipped_reward_items_missing_reward
                .len()
            + player_choice_outcome
                .report
                .skipped_reward_items_missing_item
                .len(),
        player_choice_outcome
            .report
            .skipped_reward_currencies_missing_choice
            .len()
            + player_choice_outcome
                .report
                .skipped_reward_currencies_missing_response
                .len()
            + player_choice_outcome
                .report
                .skipped_reward_currencies_missing_reward
                .len()
            + player_choice_outcome
                .report
                .skipped_reward_currencies_missing_currency
                .len(),
        player_choice_outcome
            .report
            .skipped_reward_factions_missing_choice
            .len()
            + player_choice_outcome
                .report
                .skipped_reward_factions_missing_response
                .len()
            + player_choice_outcome
                .report
                .skipped_reward_factions_missing_reward
                .len()
            + player_choice_outcome
                .report
                .skipped_reward_factions_missing_faction
                .len(),
        player_choice_outcome
            .report
            .skipped_reward_item_choices_missing_choice
            .len()
            + player_choice_outcome
                .report
                .skipped_reward_item_choices_missing_response
                .len()
            + player_choice_outcome
                .report
                .skipped_reward_item_choices_missing_reward
                .len()
            + player_choice_outcome
                .report
                .skipped_reward_item_choices_missing_item
                .len(),
        player_choice_outcome
            .report
            .skipped_maw_powers_missing_choice
            .len()
            + player_choice_outcome
                .report
                .skipped_maw_powers_missing_response
                .len(),
        player_choice_outcome.report.invalid_reward_titles.len()
            + player_choice_outcome.report.invalid_reward_packages.len()
            + player_choice_outcome
                .report
                .invalid_reward_skill_lines
                .len()
    );
    let player_choice_locale_report = crate::player_choice_catalog::load_locales_like_cpp(
        &mut player_choice_outcome.store,
        &player_choice_catalog_persistence,
    )
    .await
    .context("Failed to load C++ playerchoice_locale/playerchoice_response_locale rows")?;
    for (choice_id, locale_name) in
        &player_choice_locale_report.skipped_choice_locales_missing_choice
    {
        tracing::error!(
            target: "sql.sql",
            "Table `playerchoice_locale` references non-existing ChoiceId: {} for locale {}, skipped",
            choice_id,
            locale_name
        );
    }
    for (choice_id, response_id, locale_name) in
        &player_choice_locale_report.skipped_response_locales_missing_choice_locale
    {
        tracing::error!(
            target: "sql.sql",
            "Table `playerchoice_locale` references non-existing ChoiceId: {} for ResponseId {} locale {}, skipped",
            choice_id,
            response_id,
            locale_name
        );
    }
    for (choice_id, response_id, locale_name) in
        &player_choice_locale_report.skipped_response_locales_missing_response
    {
        tracing::error!(
            target: "sql.sql",
            "Table `playerchoice_locale` references non-existing ResponseId: {} for ChoiceId {} locale {}, skipped",
            response_id,
            choice_id,
            locale_name
        );
    }
    info!(
        "Loaded {} Player Choice locale strings ({} rows seen)",
        player_choice_locale_report.loaded_choice_locale_entries,
        player_choice_locale_report.choice_locale_rows_seen
    );
    info!(
        "Loaded {} Player Choice Response locale strings ({} rows seen)",
        player_choice_locale_report.loaded_response_locale_rows,
        player_choice_locale_report.response_locale_rows_seen
    );
    let _player_choice_store = Arc::new(player_choice_outcome.store);
    let spell_visual_store = wow_data::SpellVisualStore::load(&data_dir, &locale)
        .context("Failed to load SpellVisual.db2 for C++ jump_charge_params validation")?;
    let spell_x_spell_visual_store = Arc::new(
        spell_core_db2_hotfix::load_spell_x_spell_visual_store_like_cpp(
            &data_dir,
            &locale,
            &spell_core_hotfix_persistence,
            &db2_hotfix_removals,
        )
        .await
        .context("Failed to load effective SpellXSpellVisual authority for creature casts")?,
    );
    let jump_charge_persistence =
        wow_database::MariaDbJumpChargeCatalogPersistenceAdapterLikeCpp::new(Arc::clone(&world_db));
    let jump_charge_params_outcome = crate::jump_charge_catalog::load_jump_charge_catalog_like_cpp(
        &jump_charge_persistence,
        |id| spell_visual_store.get(id).is_some(),
        |id| curve_store.get(id).is_some(),
    )
    .await
    .context("Failed to load C++ jump_charge_params rows")?;
    for (id, speed) in &jump_charge_params_outcome.report.corrected_invalid_speeds {
        tracing::error!(
            target: "sql.sql",
            "Table `jump_charge_params` has invalid speed {} for id {}, using default {}",
            speed,
            id,
            wow_data::SPEED_CHARGE_LIKE_CPP
        );
    }
    for (id, gravity) in &jump_charge_params_outcome
        .report
        .corrected_invalid_jump_gravities
    {
        tracing::error!(
            target: "sql.sql",
            "Table `jump_charge_params` has invalid jumpGravity {} for id {}, using default {}",
            gravity,
            id,
            wow_data::MOVEMENT_GRAVITY_LIKE_CPP
        );
    }
    for (id, spell_visual_id) in &jump_charge_params_outcome
        .report
        .ignored_missing_spell_visuals
    {
        tracing::error!(
            target: "sql.sql",
            "Table `jump_charge_params` references non-existing SpellVisual {} for id {}, ignored",
            spell_visual_id,
            id
        );
    }
    for (id, progress_curve_id, cpp_logged_spell_visual_id) in &jump_charge_params_outcome
        .report
        .ignored_missing_progress_curves
    {
        tracing::error!(
            target: "sql.sql",
            "Table `jump_charge_params` references non-existing progress Curve {} for id {}, ignored (C++ log typo would print SpellVisual {:?})",
            progress_curve_id,
            id,
            cpp_logged_spell_visual_id
        );
    }
    for (id, parabolic_curve_id) in &jump_charge_params_outcome
        .report
        .ignored_missing_parabolic_curves
    {
        tracing::error!(
            target: "sql.sql",
            "Table `jump_charge_params` references non-existing parabolic Curve {} for id {}, ignored",
            parabolic_curve_id,
            id
        );
    }
    info!(
        "Loaded {} C++ jump charge params from {} rows ({} defaults applied, {} invalid optional refs ignored; live EffectJumpCharge consumption pending)",
        jump_charge_params_outcome.report.loaded_params,
        jump_charge_params_outcome.report.rows_seen,
        jump_charge_params_outcome
            .report
            .corrected_invalid_speeds
            .len()
            + jump_charge_params_outcome
                .report
                .corrected_invalid_jump_gravities
                .len(),
        jump_charge_params_outcome
            .report
            .ignored_missing_spell_visuals
            .len()
            + jump_charge_params_outcome
                .report
                .ignored_missing_progress_curves
                .len()
            + jump_charge_params_outcome
                .report
                .ignored_missing_parabolic_curves
                .len()
    );
    let _jump_charge_params_store = Arc::new(jump_charge_params_outcome.store);
    let quest_faction_reward_store = Arc::new(
        wow_data::progression_rewards::QuestFactionRewardStore::load(&data_dir, &locale).context(
            "Failed to load QuestFactionReward.db2 — check DataDir and DBC.Locale config",
        )?,
    );
    let progression_faction_store = Arc::new(
        wow_data::progression_rewards::FactionStore::load(&data_dir, &locale).context(
            "Failed to load Faction.db2 progression store — check DataDir and DBC.Locale config",
        )?,
    );
    let faction_template_store = Arc::new(
        wow_data::progression_rewards::FactionTemplateStore::load(&data_dir, &locale)
            .context("Failed to load FactionTemplate.db2 — check DataDir and DBC.Locale config")?,
    );
    let friendship_rep_reaction_store = Arc::new(
        wow_data::progression_rewards::FriendshipRepReactionStore::load(&data_dir, &locale)
            .context(
                "Failed to load FriendshipRepReaction.db2 — check DataDir and DBC.Locale config",
            )?,
    );
    let paragon_reputation_store = Arc::new(
        wow_data::progression_rewards::ParagonReputationStore::load(&data_dir, &locale).context(
            "Failed to load ParagonReputation.db2 — check DataDir and DBC.Locale config",
        )?,
    );
    let reputation_catalog_persistence =
        wow_database::MariaDbReputationCatalogPersistenceAdapterLikeCpp::new(Arc::clone(&world_db));
    let (reputation_reward_rate_store, reputation_reward_rate_report) =
        crate::reputation_catalog::load_reward_rate_store_like_cpp(
            &reputation_catalog_persistence,
            &progression_faction_store,
        )
        .await
        .context("Failed to load reputation_reward_rate")?;
    let reputation_reward_rate_store = Arc::new(reputation_reward_rate_store);
    tracing::info!(
        loaded = reputation_reward_rate_store.len(),
        skipped = reputation_reward_rate_report.skipped.len(),
        "Loaded reputation_reward_rate like C++"
    );
    let (creature_onkill_reputation_store, creature_onkill_reputation_report) =
        crate::reputation_catalog::load_creature_onkill_store_like_cpp(
            &reputation_catalog_persistence,
            &creature_template_lifecycle_store,
            &progression_faction_store,
        )
        .await
        .context("Failed to load creature_onkill_reputation")?;
    let creature_onkill_reputation_store = Arc::new(creature_onkill_reputation_store);
    tracing::info!(
        loaded = creature_onkill_reputation_store.len(),
        skipped = creature_onkill_reputation_report.skipped.len(),
        "Loaded creature_onkill_reputation like C++"
    );
    let (reputation_spillover_template_store, reputation_spillover_template_report) =
        crate::reputation_catalog::load_spillover_template_store_like_cpp(
            &reputation_catalog_persistence,
            &progression_faction_store,
        )
        .await
        .context("Failed to load reputation_spillover_template")?;
    let reputation_spillover_template_store = Arc::new(reputation_spillover_template_store);
    tracing::info!(
        loaded = reputation_spillover_template_store.len(),
        skipped = reputation_spillover_template_report.skipped.len(),
        "Loaded reputation_spillover_template like C++"
    );

    let active_realm = load_realm_info_from_snapshot_like_cpp(&realm_list, realm_id)?;
    let realm_names = realm_name_records_from_snapshot_like_cpp(&realm_list);
    let realm_build = active_realm.build;
    let win64_auth_seed = load_realm_win64_auth_seed_like_cpp(&login_db, realm_build).await?;
    info!("Realm {realm_id} build {realm_build}, Win64AuthSeed loaded");

    let realm_external_address = resolve_realm_endpoint_address_like_cpp(
        "address",
        &active_realm.address,
        &active_realm.name,
        u32::from(realm_id),
    )
    .await?;
    let realm_local_address = resolve_realm_endpoint_address_like_cpp(
        "localAddress",
        &active_realm.local_address,
        &active_realm.name,
        u32::from(realm_id),
    )
    .await?;
    info!(
        "Realm addresses: external={}, local={}",
        format_ipv4(realm_external_address),
        format_ipv4(realm_local_address),
    );

    // Share the Login DB only with account-owned composition adapters.
    let login_db = Arc::new(login_db);
    let battle_pet_account_registry = Arc::new(BattlePetAccountRegistryLikeCpp::new(
        Arc::new(LoginBattlePetPersistenceLikeCpp::new(Arc::clone(&login_db))),
        Arc::clone(&battle_pet_species_entry_store),
        Arc::clone(&battle_pet_breed_quality_store),
        Arc::clone(&battle_pet_breed_state_store),
        Arc::clone(&battle_pet_species_state_store),
        realm_id,
        active_realm.id.address_like_cpp(),
    ));

    // Build handler dispatch table
    let table = wow_world::session::registry::build_dispatch_table();
    info!("Loaded {} packet handlers", table.len());

    // Build account lookup
    let account_lookup: Arc<dyn AccountLookup> = Arc::new(DbAccountLookup {
        login_db: Arc::clone(&login_db),
        realm_id,
        win64_auth_seed,
    });

    let player_registry = Arc::new(PlayerRegistry::new());
    let active_session_registry = Arc::new(ActiveWorldSessionRegistryLikeCpp::new());
    let mut condition_load_report = crate::condition_disable_catalog::load_conditions_like_cpp(
        &condition_disable_catalog_persistence,
        |_| 0,
    )
    .await
    .context("Failed to load C++ conditions table")?;
    let loot_template_exists = |source_type: wow_constants::ConditionSourceType,
                                source_group: u32| {
        loot_store_kind_for_condition_source_type_like_cpp(source_type as i32)
            .and_then(|kind| loot_stores.get(&kind))
            .is_some_and(|store| store.have_loot_for(source_group))
    };
    let loot_source_entry_exists = |source_type: wow_constants::ConditionSourceType,
                                    source_group: u32,
                                    source_entry: i32| {
        let Some(source_entry) = u32::try_from(source_entry).ok() else {
            return false;
        };
        let Some(store) = loot_store_kind_for_condition_source_type_like_cpp(source_type as i32)
            .and_then(|kind| loot_stores.get(&kind))
        else {
            return false;
        };
        let Some(template) = store.get_loot_for(source_group) else {
            return false;
        };

        item_store.get(source_entry).is_some() || template.is_reference_like_cpp(source_entry)
    };
    let externally_skipped_conditions =
        wow_data::conditions::apply_external_condition_validation_like_cpp(
            &mut condition_load_report,
            wow_data::conditions::ConditionExternalValidationStoresLikeCpp {
                item_store: Some(item_store.as_ref()),
                spell_store: Some(&spell_store),
                area_table_store: Some(area_table_store.as_ref()),
                skill_line_store: Some(skill_line_store.as_ref()),
                map_store: Some(map_store.as_ref()),
                phase_store: Some(phase_store.as_ref()),
                quest_store: Some(quest_store.as_ref()),
                area_trigger_db2_store: Some(area_trigger_db2_store.as_ref()),
                graveyard_store: Some(&graveyard_store),
                spawn_group_store: Some(&spawn_group_store),
                creature_template_store: Some(creature_template_store.as_ref()),
                gameobject_template_store: Some(gameobject_template_store.as_ref()),
                trainer_store: Some(trainer_store.as_ref()),
                conversation_line_template_store: Some(conversation_line_template_store.as_ref()),
                area_trigger_template_store: Some(area_trigger_template_store.as_ref()),
                creature_spawn_store: Some(creature_spawn_store.as_ref()),
                gameobject_spawn_store: Some(gameobject_spawn_store.as_ref()),
                active_event_store: Some(active_event_store.as_ref()),
                world_state_store: Some(world_state_store.as_ref()),
                difficulty_store: Some(difficulty_store.as_ref()),
                faction_store: Some(faction_store.as_ref()),
                achievement_store: Some(achievement_store.as_ref()),
                char_titles_store: Some(char_titles_store.as_ref()),
                battle_pet_species_store: Some(battle_pet_species_store.as_ref()),
                scenario_step_store: Some(scenario_step_store.as_ref()),
                scene_script_package_store: Some(scene_script_package_store.as_ref()),
                player_condition_store: Some(player_condition_store.as_ref()),
                max_skill_value: Some(max_skill_value_like_cpp(&world_configs)),
                loot_template_exists: Some(&loot_template_exists),
                loot_source_entry_exists: Some(&loot_source_entry_exists),
            },
        );
    for skipped in &condition_load_report.skipped {
        warn!(
            "Condition row skipped during C++ load-shape parsing: {:?}: {:?}",
            skipped.row, skipped.reason
        );
    }
    for skipped in &externally_skipped_conditions {
        warn!(
            "Condition row skipped during C++ external validation: {:?}: {:?}",
            skipped.condition, skipped.reason
        );
    }
    for warning in &condition_load_report.warnings {
        warn!("Condition load warning: {warning:?}");
    }
    let condition_store = Arc::new(condition_load_report.into_store_like_cpp());
    let condition_attachment_report = wow_data::attach_loaded_conditions_like_cpp(
        condition_store.as_ref(),
        Some(&mut gossip_store),
        Some(&mut spell_store),
        Some(&mut phase_info_store),
        Some(&mut graveyard_store),
    );
    for missing in &condition_attachment_report.gossip_menus.missing_menus {
        warn!(
            "ConditionMgr gossip attachment warning: GossipMenu {} not found for condition id {:?}",
            missing.source_group, missing
        );
    }
    for missing in &condition_attachment_report
        .gossip_menu_items
        .missing_menu_items
    {
        warn!(
            "ConditionMgr gossip attachment warning: GossipMenuId {} Item {} not found for condition id {:?}",
            missing.source_group, missing.source_entry, missing
        );
    }
    info!(
        "Loaded C++ ConditionMgr store: {} buckets, {} externally skipped conditions, {} spell-click aura spell ids, {} spell implicit target condition rows attached ({} deferred), {} gossip menu condition rows attached ({} missing menus), {} gossip menu option condition rows attached ({} missing items), {} phase condition rows attached, {} graveyard condition rows attached",
        condition_store.bucket_count(),
        externally_skipped_conditions.len(),
        condition_attachment_report.spell_click_aura_spell_ids.len(),
        condition_attachment_report.spell_implicit_target_condition_count,
        condition_attachment_report.deferred_spell_implicit_target_condition_count,
        condition_attachment_report
            .gossip_menus
            .attached_condition_count,
        condition_attachment_report.gossip_menus.missing_menus.len(),
        condition_attachment_report
            .gossip_menu_items
            .attached_condition_count,
        condition_attachment_report
            .gossip_menu_items
            .missing_menu_items
            .len(),
        condition_attachment_report.phases.attached_condition_count,
        condition_attachment_report
            .graveyards
            .attached_condition_count
    );
    wow_world::conditions::set_condition_mgr_store_like_cpp(Arc::clone(&condition_store));
    let graveyard_store = Arc::new(graveyard_store);
    let npc_spell_click_store = Arc::new(
        crate::gameplay_rule_catalog::load_npc_spell_click_store_like_cpp(
            &gameplay_rule_catalog_persistence,
            creature_template_lifecycle_store.as_ref(),
            &spell_store,
        )
        .await
        .context("Failed to load C++ npc_spellclick_spells rows")?,
    );
    let spellclick_templates_without_data = npc_spell_click_store
        .templates_with_spellclick_flag_but_no_data_like_cpp(
            creature_template_lifecycle_store
                .entries_like_cpp()
                .map(|template| (template.entry, template.npc_flags)),
        );
    let spellclick_template_flags_removed = Arc::make_mut(&mut creature_template_lifecycle_store)
        .remove_npc_flag_for_entries_like_cpp(
            spellclick_templates_without_data.iter().copied(),
            wow_data::UNIT_NPC_FLAG_SPELLCLICK_LIKE_CPP,
        );
    info!(
        "Loaded {} C++ npc_spellclick_spells rows ({} missing creature templates, {} missing spells, {} invalid user types logged-but-loaded like C++, {} templates with UNIT_NPC_FLAG_SPELLCLICK but no data, {} flags removed)",
        npc_spell_click_store.len(),
        npc_spell_click_store
            .load_report_like_cpp()
            .skipped_missing_creature_template,
        npc_spell_click_store
            .load_report_like_cpp()
            .skipped_missing_spell,
        npc_spell_click_store
            .load_report_like_cpp()
            .invalid_user_type_logged_but_loaded_like_cpp,
        spellclick_templates_without_data.len(),
        spellclick_template_flags_removed
    );
    let spell_target_position_store = Arc::new(
        crate::spell_world_catalog::load_spell_target_position_like_cpp(
            &spell_world_catalog_persistence,
            &spell_store,
            |map_id| map_store.get(u32::from(map_id)).is_some(),
        )
        .await
        .context("Failed to load C++ spell_target_position rows")?,
    );
    info!(
        "Loaded {} C++ spell_target_position rows ({} missing maps, {} missing spells, {} missing effects, {} zero positions, {} unsupported target rows skipped)",
        spell_target_position_store.len(),
        spell_target_position_store
            .load_report_like_cpp()
            .skipped_missing_map,
        spell_target_position_store
            .load_report_like_cpp()
            .skipped_missing_spell,
        spell_target_position_store
            .load_report_like_cpp()
            .skipped_missing_effect,
        spell_target_position_store
            .load_report_like_cpp()
            .skipped_zero_position,
        spell_target_position_store
            .load_report_like_cpp()
            .skipped_unsupported_target
    );
    let spell_proc_outcome = crate::spell_world_catalog::load_spell_proc_like_cpp(
        &spell_world_catalog_persistence,
        &spell_store,
        spell_chain_store.as_ref(),
        spell_aura_options_store.as_ref(),
        spell_misc_store.as_ref(),
        spell_class_options_store.as_ref(),
        spell_procs_per_minute_store.as_ref(),
    )
    .await
    .context("Failed to load C++ spell_proc rows")?;
    let spell_proc_store = Arc::new(spell_proc_outcome.store);
    info!(
        "Loaded {} C++ spell_proc rows and generated {} implicit spell proc entries ({} validation issues)",
        spell_proc_outcome.loaded_row_count,
        spell_proc_outcome.generated_entry_count,
        spell_proc_outcome.errors.len()
    );
    let spell_required_outcome = crate::spell_world_catalog::load_spell_required_like_cpp(
        &spell_world_catalog_persistence,
        &spell_store,
        spell_chain_store.as_ref(),
    )
    .await
    .context("Failed to load C++ spell_required rows")?;
    let spell_required_store = Arc::new(spell_required_outcome.store);
    info!(
        "Loaded {} C++ spell_required rows ({} validation issues)",
        spell_required_outcome.loaded_row_count,
        spell_required_outcome.errors.len()
    );
    let spell_group_outcome = crate::spell_world_catalog::load_spell_group_like_cpp(
        &spell_world_catalog_persistence,
        &spell_store,
        spell_chain_store.as_ref(),
    )
    .await
    .context("Failed to load C++ spell_group rows")?;
    let spell_group_store = Arc::new(spell_group_outcome.store);
    info!(
        "Loaded {} C++ spell_group expanded definitions ({} validation issues)",
        spell_group_outcome.loaded_row_count,
        spell_group_outcome.errors.len()
    );
    let spell_group_stack_rule_outcome =
        crate::spell_world_catalog::load_spell_group_stack_rule_like_cpp(
            &spell_world_catalog_persistence,
            spell_group_store.as_ref(),
            &spell_store,
            spell_chain_store.as_ref(),
        )
        .await
        .context("Failed to load C++ spell_group_stack_rules rows")?;
    let spell_group_stack_rule_store = Arc::new(spell_group_stack_rule_outcome.store);
    info!(
        "Loaded {} C++ spell_group_stack_rules rows and parsed {} same-effect groups ({} validation issues)",
        spell_group_stack_rule_outcome.loaded_row_count,
        spell_group_stack_rule_outcome.same_effect_parsed_count,
        spell_group_stack_rule_outcome.errors.len()
    );
    let spell_threat_outcome = crate::spell_world_catalog::load_spell_threat_like_cpp(
        &spell_world_catalog_persistence,
        &spell_store,
    )
    .await
    .context("Failed to load C++ spell_threat rows")?;
    let spell_threat_store = Arc::new(spell_threat_outcome.store);
    info!(
        "Loaded {} C++ spell_threat rows ({} missing spells)",
        spell_threat_outcome.loaded_row_count,
        spell_threat_outcome.errors.len()
    );
    let spell_linked_outcome = crate::spell_world_catalog::load_spell_linked_like_cpp(
        &spell_world_catalog_persistence,
        &spell_store,
    )
    .await
    .context("Failed to load C++ spell_linked_spell rows")?;
    let spell_linked_rejected_trigger_spell_ids = Arc::new(
        spell_linked_outcome
            .errors
            .iter()
            .map(|error| error.row.spell_trigger.unsigned_abs())
            .collect::<BTreeSet<_>>(),
    );
    let spell_linked_store = Arc::new(spell_linked_outcome.store);
    info!(
        "Loaded {} C++ spell_linked_spell rows ({} validation issues, {} warnings)",
        spell_linked_outcome.loaded_row_count,
        spell_linked_outcome.errors.len(),
        spell_linked_outcome.warnings.len()
    );
    let spell_totem_model_outcome = crate::spell_world_catalog::load_spell_totem_model_like_cpp(
        &spell_world_catalog_persistence,
        |spell_id| spell_store.get(spell_id as i32).is_some(),
        |race_id| chr_races_store.get(u32::from(race_id)).is_some(),
        |display_id| creature_display_info_store.get(display_id).is_some(),
    )
    .await
    .context("Failed to load C++ spell_totem_model rows")?;
    info!(
        "Loaded {} C++ spell_totem_model rows ({} validation issues)",
        spell_totem_model_outcome.loaded_row_count,
        spell_totem_model_outcome.errors.len()
    );
    let spell_pet_aura_outcome = crate::spell_world_catalog::load_spell_pet_aura_like_cpp(
        &spell_world_catalog_persistence,
        &spell_store,
    )
    .await
    .context("Failed to load C++ spell_pet_auras rows")?;
    let spell_pet_aura_store = Arc::new(spell_pet_aura_outcome.store);
    info!(
        "Loaded {} C++ spell_pet_auras rows ({} validation issues)",
        spell_pet_aura_outcome.loaded_row_count,
        spell_pet_aura_outcome.errors.len()
    );
    let trainer_spell_static_authority =
        spell_acquisition_loader::load_trainer_static_authority_like_cpp(
            &data_dir,
            &locale,
            &spell_acquisition_startup_persistence,
            &db2_hotfix_removals,
            &spell_store,
            spell_chain_store.as_ref(),
            spell_acquisition_catalog.as_ref(),
            spell_linked_store.as_ref(),
            spell_pet_aura_store.as_ref(),
            spell_aura_restrictions_store.as_ref(),
            spell_casting_requirements_store.as_ref(),
            spell_equipped_items_store.as_ref(),
            spell_area_store.as_ref(),
            |item_id| item_stats_store.sparse_template(item_id).is_some(),
        )
        .await
        .context("Failed to audit normal trainer wrapper authority")?;
    let spell_acquisition_safe_cast_spell_ids =
        Arc::new(trainer_spell_static_authority.safe_cast_spell_ids);
    let spell_acquisition_valid_craft_spell_ids =
        Arc::new(trainer_spell_static_authority.valid_craft_spell_ids);
    let spell_script_exact_spell_ids =
        Arc::new(trainer_spell_static_authority.spell_script_exact_spell_ids);
    let spell_script_all_rank_root_spell_ids =
        Arc::new(trainer_spell_static_authority.spell_script_all_rank_root_spell_ids);
    let legacy_spell_script_spell_ids =
        Arc::new(trainer_spell_static_authority.legacy_spell_script_spell_ids);
    info!(
        safe_cast_count = spell_acquisition_safe_cast_spell_ids.len(),
        valid_craft_count = spell_acquisition_valid_craft_spell_ids.len(),
        spell_script_exact_count = spell_script_exact_spell_ids.len(),
        spell_script_all_rank_count = spell_script_all_rank_root_spell_ids.len(),
        legacy_spell_script_count = legacy_spell_script_spell_ids.len(),
        "Loaded fail-closed normal trainer spell-acquisition authority"
    );
    let spell_store = Arc::new(spell_store);

    // Shared group registry and pending invites
    let group_registry = Arc::new(GroupRegistry::new());
    let pending_invites = Arc::new(PendingInvites::new());
    let represented_group_persistence_adapter = Arc::new(
        wow_database::represented_group_persistence_adapter::MariaDbRepresentedGroupPersistenceAdapterLikeCpp::new(
            Arc::clone(&char_db),
        ),
    );
    let group_load_summary = load_groups_from_character_database_like_cpp(
        represented_group_persistence_adapter.as_ref(),
        group_registry.as_ref(),
        difficulty_store.as_ref(),
    )
    .await
    .context("Failed to load C++ group startup state")?;
    info!(
        "Loaded C++ group startup state: groups={} member-rows={} members={} skipped-groups={} skipped-members={}",
        group_load_summary.loaded_groups,
        group_load_summary.loaded_member_rows,
        group_load_summary.loaded_members,
        group_load_summary.skipped_group_rows,
        group_load_summary.skipped_member_rows,
    );

    // Shared world state (creatures/grids visible to every session on the same map).
    // Each session gets a clone of this Arc on creation.
    let mut legacy_map_manager = LegacyMapManager::new();
    // Wire file-backed terrain so the live spawn/respawn path ground-snaps
    // creatures with real `.map` heights (issue #15). DataDir-rooted, lazy.
    legacy_map_manager.set_terrain(Arc::new(wow_world::map_manager::LiveTerrainHeights::new(
        &data_dir,
    )));
    let shared_map: SharedMapManager = Arc::new(std::sync::RwLock::new(legacy_map_manager));
    let instance_lock_persistence_port: Arc<
        dyn wow_persistence::InstanceLockPersistencePortLikeCpp,
    > = Arc::new(
        wow_database::MariaDbInstanceLockPersistenceAdapterLikeCpp::new(Arc::clone(&char_db)),
    );
    let (shared_instance_lock_rows, character_instance_lock_rows) =
        match instance_lock_persistence_port.load_all_like_cpp().await {
            wow_persistence::InstanceLockPersistenceLoadOutcomeLikeCpp::Loaded {
                shared_rows,
                character_rows,
            } => (shared_rows, character_rows),
            wow_persistence::InstanceLockPersistenceLoadOutcomeLikeCpp::Failed { reason } => {
                bail!("Failed to load instance locks from character database: {reason}")
            }
        };
    let mut loaded_instance_lock_mgr = InstanceLockMgr::default();
    let instance_lock_load_issues = loaded_instance_lock_mgr.load_from_rows_like_cpp(
        shared_instance_lock_rows,
        character_instance_lock_rows,
        |map_id, difficulty_id| {
            map_db2_entries_from_stores(&map_store, &map_difficulty_store, map_id, difficulty_id)
        },
    );
    for issue in &instance_lock_load_issues {
        warn!("Instance lock load issue: {issue:?}");
    }
    let instance_lock_stats = loaded_instance_lock_mgr.statistics();
    info!(
        "Loaded instance locks: {} shared instances, {} players, {} issues",
        instance_lock_stats.instance_count,
        instance_lock_stats.player_count,
        instance_lock_load_issues.len()
    );
    let registered_instance_ids = loaded_instance_lock_mgr.registered_instance_ids_like_cpp_order();
    let instance_lock_mgr = Arc::new(std::sync::RwLock::new(loaded_instance_lock_mgr));

    let canonical_map_manager = Arc::new(Mutex::new(create_canonical_map_manager(&world_configs)));
    assert!(player_registry.bind_canonical_map_manager(Arc::clone(&canonical_map_manager)));
    match canonical_map_manager.lock() {
        Ok(mut manager) => install_canonical_spawn_group_initializer_like_cpp(
            &mut manager,
            Arc::clone(&canonical_spawn_metadata),
            Arc::clone(&condition_store),
            Arc::clone(&persisted_respawn_times),
            Arc::clone(&map_store),
        ),
        Err(_) => {
            warn!("Canonical MapManager lock poisoned; InitSpawnGroupState hook not installed")
        }
    }
    register_loaded_instance_ids(
        &shared_map,
        canonical_map_manager.as_ref(),
        &registered_instance_ids,
    );

    let loaded_grid_creature_respawn_caches = LoadedGridCreatureRespawnCachesLikeCpp {
        realm_id,
        template_store: Arc::clone(&creature_template_lifecycle_store),
        sparring_store: Arc::clone(&creature_template_sparring_store),
        difficulty_store: Arc::clone(&creature_difficulty_store),
        base_stats_store: Arc::clone(&creature_base_stats_store),
        chr_classes_store: Arc::clone(&chr_classes_store),
        power_type_store: Arc::clone(&power_type_store),
        health_rates: creature_health_rates,
        display_store: Arc::clone(&creature_display_info_store),
        model_store: Arc::clone(&creature_model_data_store),
        model_info_store: Arc::clone(&creature_model_info_store),
        creature_equipment_store: Arc::clone(&creature_equipment_store),
        creature_addon_store: Arc::clone(&creature_addon_store),
        vehicle_store: Arc::clone(&vehicle_store),
        vehicle_seat_store: Arc::clone(&vehicle_seat_store),
        vehicle_accessory_store: Arc::clone(&vehicle_accessory_store),
        gameobject_template_store: Arc::clone(&gameobject_template_lifecycle_store),
        gameobject_override_store: Arc::clone(&gameobject_override_lifecycle_store),
    };

    let game_event_scheduler = {
        let current_time_secs = current_unix_time_secs_like_cpp();
        let (game_event_outcome, active_event_ids, mut db_bridge_summary) = {
            let mut canonical_spawn_metadata = canonical_spawn_metadata.lock().map_err(|_| {
                anyhow::anyhow!(
                    "CanonicalSpawnMetadataLikeCpp mutex poisoned during GameEvent StartSystem"
                )
            })?;
            canonical_spawn_metadata.clear_active_game_events_like_cpp();
            let outcome = canonical_spawn_metadata.update_game_events_like_cpp(
                current_time_secs,
                false,
                represented_game_event_world_conditions_met_like_cpp,
            );
            let db_bridge_summary = materialize_game_event_world_event_state_db_bridge_like_cpp(
                &outcome,
                &canonical_spawn_metadata,
            );
            let active_event_ids = canonical_spawn_metadata
                .game_event_active_set_like_cpp()
                .active_event_ids_like_cpp()
                .collect::<Vec<_>>();
            (outcome, active_event_ids, db_bridge_summary)
        };
        execute_game_event_world_event_state_db_bridge_like_cpp(
            game_event_persistence.as_ref(),
            &mut db_bridge_summary,
        )
        .await;
        let mut side_effect_summary = {
            let mut manager = canonical_map_manager.lock().map_err(|_| {
                anyhow::anyhow!("Canonical MapManager mutex poisoned during GameEvent StartSystem")
            })?;
            let mut canonical_spawn_metadata = canonical_spawn_metadata.lock().map_err(|_| {
                anyhow::anyhow!("CanonicalSpawnMetadataLikeCpp mutex poisoned during GameEvent StartSystem side effects")
            })?;
            let mut world_state_mgr = world_state_mgr.lock().map_err(|_| {
                anyhow::anyhow!(
                    "WorldStateMgrLikeCpp mutex poisoned during GameEvent StartSystem side effects"
                )
            })?;
            consume_game_event_live_update_side_effects_like_cpp(
                &mut manager,
                Some(&shared_map),
                &mut canonical_spawn_metadata,
                &loaded_grid_creature_respawn_caches,
                Some(battlemaster_list_typed_store.as_ref()),
                Some(&mut world_state_mgr),
                Some(player_registry.as_ref()),
                &active_event_ids,
                &game_event_outcome,
                false,
            )
        };
        execute_game_event_seasonal_quest_db_deletes_like_cpp(
            game_event_persistence.as_ref(),
            &mut side_effect_summary,
        )
        .await;
        fanout_reset_event_seasonal_quests_to_player_sessions_after_db_delete_like_cpp(
            Some(player_registry.as_ref()),
            &mut side_effect_summary,
        );
        debug!(
            scanned_event_ids = game_event_outcome.scanned_event_ids.len(),
            queued_activation_event_ids = game_event_outcome.queued_activation_event_ids.len(),
            queued_deactivation_event_ids = game_event_outcome.queued_deactivation_event_ids.len(),
            start_outcomes = game_event_outcome.start_outcomes.len(),
            stop_outcomes = game_event_outcome.stop_outcomes.len(),
            negative_spawn_event_ids = game_event_outcome.negative_spawn_event_ids.len(),
            world_nextphase_finished = game_event_outcome.world_nextphase_finished.len(),
            world_conditions_save_requested =
                game_event_outcome.world_conditions_save_requested.len(),
            game_event_db_saves_queued = db_bridge_summary.saves_queued,
            game_event_db_saves_executed = db_bridge_summary.saves_executed,
            game_event_db_saves_failed = db_bridge_summary.saves_failed,
            game_event_db_saves_skipped_event_id_out_of_range =
                db_bridge_summary.saves_skipped_event_id_out_of_range,
            game_event_db_saves_skipped_missing_event =
                db_bridge_summary.saves_skipped_missing_event,
            game_event_db_deletes_queued = db_bridge_summary.deletes_queued,
            game_event_db_deletes_executed = db_bridge_summary.deletes_executed,
            game_event_db_deletes_failed = db_bridge_summary.deletes_failed,
            game_event_db_deletes_skipped_event_id_out_of_range =
                db_bridge_summary.deletes_skipped_event_id_out_of_range,
            game_event_db_condition_delete_rows_queued =
                db_bridge_summary.condition_delete_rows_queued,
            game_event_db_condition_delete_rows_executed =
                db_bridge_summary.condition_delete_rows_executed,
            game_event_db_condition_delete_rows_failed =
                db_bridge_summary.condition_delete_rows_failed,
            invalid_check_outcomes = game_event_outcome.invalid_check_outcomes.len(),
            invalid_next_check_outcomes = game_event_outcome.invalid_next_check_outcomes.len(),
            next_update_delay_millis = game_event_outcome.next_update_delay_millis,
            side_effect_actions = side_effect_summary.actions.len(),
            spawn_actions = side_effect_summary.spawn_actions,
            unspawn_actions = side_effect_summary.unspawn_actions,
            announce_event_actions = side_effect_summary.announce_event_actions,
            announce_event_description_len_total =
                side_effect_summary.announce_event_description_len_total,
            announce_event_world_text_represented =
                side_effect_summary.announce_event_world_text_represented,
            announce_event_lines = side_effect_summary.announce_event_lines,
            announce_event_registry_missing = side_effect_summary.announce_event_registry_missing,
            announce_event_send_attempted = side_effect_summary.announce_event_send_attempted,
            announce_event_send_queued = side_effect_summary.announce_event_send_queued,
            announce_event_send_failed = side_effect_summary.announce_event_send_failed,
            announce_event_localization_unrepresented =
                side_effect_summary.announce_event_localization_unrepresented,
            announce_event_in_world_filter_unrepresented =
                side_effect_summary.announce_event_in_world_filter_unrepresented,
            announce_event_not_in_world_skipped =
                side_effect_summary.announce_event_not_in_world_skipped,
            announce_event_world_text_unimplemented =
                side_effect_summary.announce_event_world_text_unimplemented,
            announce_event_session_fanout_unimplemented =
                side_effect_summary.announce_event_session_fanout_unimplemented,
            change_equip_or_model_actions = side_effect_summary.change_equip_or_model_actions,
            change_equip_or_model_records_seen =
                side_effect_summary.change_equip_or_model_records_seen,
            change_equip_or_model_records_applied =
                side_effect_summary.change_equip_or_model_records_applied,
            change_equip_or_model_maps_matched =
                side_effect_summary.change_equip_or_model_maps_matched,
            change_equip_or_model_live_creatures_mutated =
                side_effect_summary.change_equip_or_model_live_creatures_mutated,
            change_equip_or_model_model_validation_unavailable =
                side_effect_summary.change_equip_or_model_model_validation_unavailable,
            update_event_quests_actions = side_effect_summary.update_event_quests_actions,
            update_event_quests_creature_records_seen =
                side_effect_summary.update_event_quests_creature_records_seen,
            update_event_quests_gameobject_records_seen =
                side_effect_summary.update_event_quests_gameobject_records_seen,
            update_event_quests_creature_inserted =
                side_effect_summary.update_event_quests_creature_inserted,
            update_event_quests_gameobject_inserted =
                side_effect_summary.update_event_quests_gameobject_inserted,
            update_event_quests_creature_removed =
                side_effect_summary.update_event_quests_creature_removed,
            update_event_quests_gameobject_removed =
                side_effect_summary.update_event_quests_gameobject_removed,
            update_event_quests_creature_skipped_active_other_event =
                side_effect_summary.update_event_quests_creature_skipped_active_other_event,
            update_event_quests_gameobject_skipped_active_other_event =
                side_effect_summary.update_event_quests_gameobject_skipped_active_other_event,
            update_world_states_actions = side_effect_summary.update_world_states_actions,
            update_world_states_no_holiday = side_effect_summary.update_world_states_no_holiday,
            update_world_states_missing_event =
                side_effect_summary.update_world_states_missing_event,
            update_world_states_store_missing = side_effect_summary.update_world_states_store_missing,
            update_world_states_holiday_not_weekend_battleground =
                side_effect_summary.update_world_states_holiday_not_weekend_battleground,
            update_world_states_battlemaster_list_missing =
                side_effect_summary.update_world_states_battlemaster_list_missing,
            update_world_states_holiday_world_state_zero =
                side_effect_summary.update_world_states_holiday_world_state_zero,
            update_world_states_holiday_lookup_unrepresented =
                side_effect_summary.update_world_states_holiday_lookup_unrepresented,
            update_world_states_set_value_represented =
                side_effect_summary.update_world_states_set_value_represented,
            update_world_states_last_world_state_id =
                side_effect_summary.update_world_states_last_world_state_id,
            update_world_states_last_world_state_value =
                side_effect_summary.update_world_states_last_world_state_value,
            update_npc_flags_actions = side_effect_summary.update_npc_flags_actions,
            update_npc_flags_records_seen = side_effect_summary.update_npc_flags_records_seen,
            update_npc_flags_maps_matched = side_effect_summary.update_npc_flags_maps_matched,
            update_npc_flags_live_creatures_mutated =
                side_effect_summary.update_npc_flags_live_creatures_mutated,
            update_npc_flags2_applied =
                side_effect_summary.update_npc_flags2_applied,
            update_npc_vendor_actions = side_effect_summary.update_npc_vendor_actions,
            update_npc_vendor_records_seen = side_effect_summary.update_npc_vendor_records_seen,
            update_npc_vendor_items_added = side_effect_summary.update_npc_vendor_items_added,
            update_npc_vendor_items_removed = side_effect_summary.update_npc_vendor_items_removed,
            update_npc_vendor_missing_event_buckets =
                side_effect_summary.update_npc_vendor_missing_event_buckets,
            update_npc_vendor_remove_misses = side_effect_summary.update_npc_vendor_remove_misses,
            update_npc_vendor_no_match = side_effect_summary.update_npc_vendor_no_match,
            reset_event_seasonal_quests_actions =
                side_effect_summary.reset_event_seasonal_quests_actions,
            reset_event_seasonal_quests_event_start_time_zero =
                side_effect_summary.reset_event_seasonal_quests_event_start_time_zero,
            reset_event_seasonal_quests_event_start_time_nonzero =
                side_effect_summary.reset_event_seasonal_quests_event_start_time_nonzero,
            reset_event_seasonal_quests_player_session_runtime_unimplemented = side_effect_summary
                .reset_event_seasonal_quests_player_session_runtime_unimplemented,
            reset_event_seasonal_quests_character_db_statement_unimplemented = side_effect_summary
                .reset_event_seasonal_quests_character_db_statement_unimplemented,
            reset_event_seasonal_quests_character_db_delete_queued = side_effect_summary
                .reset_event_seasonal_quests_character_db_delete_queued,
            reset_event_seasonal_quests_character_db_delete_executed = side_effect_summary
                .reset_event_seasonal_quests_character_db_delete_executed,
            reset_event_seasonal_quests_character_db_delete_failed = side_effect_summary
                .reset_event_seasonal_quests_character_db_delete_failed,
            reset_event_seasonal_quests_character_db_delete_skipped_event_start_time_out_of_range = side_effect_summary
                .reset_event_seasonal_quests_character_db_delete_skipped_event_start_time_out_of_range,
            "Represented C++ GameEventMgr::StartSystem: cleared active events, ran first Update with isSystemInit=false, installed WUPDATE_EVENTS delay, and consumed safe represented GameEventSpawn/GameEventUnspawn plus bounded ChangeEquipOrModel, UpdateEventQuests cache, represented UpdateWorldStates HolidayWorldState -> WorldStateMgr::SetValue evidence, UpdateEventNPCFlags, UpdateEventNPCVendor cache, RunSmartAIScripts evidence, ResetEventSeasonalQuests character DB delete bridge, and represented announcement evidence-only side effects; real SendWorldText/session fanout, full ConditionMgr world-event runtime, quest packets/session gossip refresh, full ObjectMgr quest runtime, real WorldStateMgr storage/session fanout/login/GM worldstate, SmartAI script dispatch, and Player/session seasonal quest reset remain pending"
        );
        CanonicalGameEventSchedulerLikeCpp::start_system(
            game_event_outcome.next_update_delay_millis,
        )
    };

    let (game_event_quest_complete_tx, game_event_quest_complete_rx) = flume::bounded(1024);
    let game_event_quest_complete_handle =
        tokio::spawn(run_game_event_quest_complete_processor_like_cpp(
            game_event_quest_complete_rx,
            Arc::clone(&canonical_spawn_metadata),
            Arc::clone(&game_event_persistence),
        ));

    let world_listener_policy = WorldListenerPolicyLikeCpp {
        max_overspeed_pings: world_config_u32(&world_configs, "CONFIG_MAX_OVERSPEED_PINGS", 2),
        socket_timeouts: SocketTimeoutsLikeCpp {
            unauthenticated_secs: u64::from(world_config_u32(
                &world_configs,
                "CONFIG_SOCKET_TIMEOUTTIME",
                900,
            )),
            active_secs: u64::from(world_config_u32(
                &world_configs,
                "CONFIG_SOCKET_TIMEOUTTIME_ACTIVE",
                60,
            )),
        },
        ip_location_store: Some(Arc::clone(&ip_location_store)),
    };

    // The Player lifecycle port is composed here, before any session is
    // accepted, so a build that cannot persist lifecycle state fails at
    // startup rather than silently dropping offline marks at logout (#200).
    let player_lifecycle_port: Arc<dyn wow_persistence::PlayerLifecyclePortLikeCpp> = Arc::new(
        wow_database::player_lifecycle_adapter::MariaDbPlayerLifecycleAdapterLikeCpp::new(
            Arc::clone(&char_db),
            Arc::clone(&login_db),
            Arc::clone(&world_db),
        ),
    );
    let character_administration_persistence_port: Arc<
        dyn wow_persistence::CharacterAdministrationPersistencePortLikeCpp,
    > = Arc::new(
        wow_database::MariaDbCharacterAdministrationPersistenceAdapterLikeCpp::new(
            Arc::clone(&char_db),
            Arc::clone(&world_db),
        ),
    );
    let character_enumeration_persistence_port: Arc<
        dyn wow_persistence::CharacterEnumerationPersistencePortLikeCpp,
    > = Arc::new(
        wow_database::MariaDbCharacterEnumerationPersistenceAdapterLikeCpp::new(Arc::clone(
            &char_db,
        )),
    );
    let item_template_addon_catalog_persistence_port: Arc<
        dyn wow_persistence::ItemTemplateAddonCatalogPersistencePortLikeCpp,
    > = Arc::new(
        wow_database::MariaDbItemTemplateAddonCatalogPersistenceAdapterLikeCpp::new(Arc::clone(
            &world_db,
        )),
    );
    let loot_template_catalog_persistence_port: Arc<
        dyn wow_persistence::LootTemplateCatalogPersistencePortLikeCpp,
    > = Arc::new(
        wow_database::MariaDbLootTemplateCatalogPersistenceAdapterLikeCpp::new(Arc::clone(
            &world_db,
        )),
    );
    let vendor_catalog_persistence_port: Arc<
        dyn wow_persistence::VendorCatalogPersistencePortLikeCpp,
    > = Arc::new(
        wow_database::MariaDbVendorCatalogPersistenceAdapterLikeCpp::new(Arc::clone(&world_db)),
    );
    let visibility_spawn_catalog_persistence_port: Arc<
        dyn wow_persistence::VisibilitySpawnCatalogPersistencePortLikeCpp,
    > = Arc::new(
        wow_database::MariaDbVisibilitySpawnCatalogPersistenceAdapterLikeCpp::new(Arc::clone(
            &world_db,
        )),
    );
    let gossip_catalog_persistence_port: Arc<
        dyn wow_persistence::GossipCatalogPersistencePortLikeCpp,
    > = gossip_catalog_adapter.clone();
    let player_name_query_persistence_port: Arc<
        dyn wow_persistence::PlayerNameQueryPersistencePortLikeCpp,
    > = Arc::new(
        wow_database::MariaDbPlayerNameQueryPersistenceAdapterLikeCpp::new(Arc::clone(&char_db)),
    );
    let session_account_state_port: Arc<dyn wow_persistence::SessionAccountStatePortLikeCpp> =
        Arc::new(
            wow_database::session_account_state_adapter::MariaDbSessionAccountStateAdapterLikeCpp::new(
                Arc::clone(&char_db),
            ),
        );
    let packet_spoof_ban_persistence_port: Arc<
        dyn wow_persistence::PacketSpoofBanPersistencePortLikeCpp,
    > = Arc::new(
        wow_database::packet_spoof_ban_adapter::MariaDbPacketSpoofBanPersistenceAdapterLikeCpp::new(
            Arc::clone(&login_db),
        ),
    );
    let void_storage_persistence_port: Arc<dyn wow_persistence::VoidStoragePersistencePortLikeCpp> =
        Arc::new(
            wow_database::void_storage_adapter::MariaDbVoidStoragePersistenceAdapterLikeCpp::new(
                Arc::clone(&char_db),
            ),
        );
    let social_persistence_port: Arc<dyn wow_persistence::SocialPersistencePortLikeCpp> = Arc::new(
        wow_database::social_adapter::MariaDbSocialPersistenceAdapterLikeCpp::new(Arc::clone(
            &char_db,
        )),
    );
    let map_corpse_persistence_port: Arc<dyn wow_persistence::MapCorpsePersistencePortLikeCpp> =
        Arc::new(
            wow_database::map_corpse_adapter::MariaDbMapCorpsePersistenceAdapterLikeCpp::new(
                Arc::clone(&char_db),
            ),
        );
    let quest_poi_persistence_port: Arc<dyn wow_persistence::QuestPoiPersistencePortLikeCpp> =
        Arc::new(
            wow_database::quest_poi_adapter::MariaDbQuestPoiPersistenceAdapterLikeCpp::new(
                Arc::clone(&world_db),
            ),
        );
    let stored_item_money_persistence_port: Arc<
        dyn wow_persistence::StoredItemMoneyPersistencePortLikeCpp,
    > = Arc::new(
        wow_database::stored_item_money_adapter::MariaDbStoredItemMoneyPersistenceAdapterLikeCpp::new(
            Arc::clone(&char_db),
        ),
    );
    let group_loot_money_persistence_port: Arc<
        dyn wow_persistence::GroupLootMoneyPersistencePortLikeCpp,
    > = Arc::new(
        wow_database::group_loot_money_adapter::MariaDbGroupLootMoneyPersistenceAdapterLikeCpp::new(
            Arc::clone(&char_db),
        ),
    );
    let represented_group_persistence_port: Arc<
        dyn wow_persistence::RepresentedGroupPersistencePortLikeCpp,
    > = represented_group_persistence_adapter;
    let support_bug_report_persistence_port: Arc<
        dyn wow_persistence::SupportBugReportPersistencePortLikeCpp,
    > = Arc::new(
        wow_database::support_bug_report_adapter::MariaDbSupportBugReportPersistenceAdapterLikeCpp::new(
            Arc::clone(&char_db),
        ),
    );
    let spell_acquisition_port = spell_acquisition_port(Arc::clone(&char_db));
    let battle_pet_purchase_persistence_port: Arc<
        dyn wow_persistence::BattlePetPurchasePersistencePortLikeCpp,
    > = Arc::new(
        wow_database::CharacterBattlePetPurchasePersistenceAdapterLikeCpp::new(Arc::clone(
            &char_db,
        )),
    );
    // Build session resources
    let stored_item_persistence_port: Arc<dyn wow_persistence::StoredItemPersistencePortLikeCpp> =
        Arc::new(
            wow_database::MariaDbStoredItemPersistenceAdapterLikeCpp::new(Arc::clone(&char_db)),
        );
    let player_inventory_persistence_port: Arc<
        dyn wow_persistence::PlayerInventoryPersistencePortLikeCpp,
    > = Arc::new(
        wow_database::MariaDbPlayerInventoryPersistenceAdapterLikeCpp::new(Arc::clone(&char_db)),
    );
    let player_quest_persistence_port: Arc<dyn wow_persistence::PlayerQuestPersistencePortLikeCpp> =
        Arc::new(
            wow_database::MariaDbPlayerQuestPersistenceAdapterLikeCpp::new(Arc::clone(&char_db)),
        );
    let vendor_trade_persistence_port: Arc<dyn wow_persistence::VendorTradePersistencePortLikeCpp> =
        Arc::new(
            wow_database::MariaDbVendorTradePersistenceAdapterLikeCpp::new(Arc::clone(&char_db)),
        );
    let persistence = wow_world::session::SessionPersistencePortsLikeCpp::required_like_cpp(
        wow_world::session::SessionAdmissionPersistenceLikeCpp::required_like_cpp(
            Arc::clone(&character_administration_persistence_port),
            Arc::clone(&character_enumeration_persistence_port),
            Arc::clone(&session_account_state_port),
            Arc::clone(&packet_spoof_ban_persistence_port),
            Arc::clone(&player_name_query_persistence_port),
            Arc::clone(&support_bug_report_persistence_port),
        ),
        wow_world::session::PlayerPersistenceCapabilitiesLikeCpp::required_like_cpp(
            Arc::clone(&player_lifecycle_port),
            Arc::clone(&void_storage_persistence_port),
            Arc::clone(&social_persistence_port),
            Arc::clone(&stored_item_money_persistence_port),
            Arc::clone(&stored_item_persistence_port),
            Arc::clone(&player_inventory_persistence_port),
            Arc::clone(&player_quest_persistence_port),
            Arc::clone(&vendor_trade_persistence_port),
            Arc::clone(&spell_acquisition_port),
            Arc::clone(&instance_lock_persistence_port),
            Arc::clone(&battle_pet_purchase_persistence_port),
        ),
        wow_world::session::WorldPersistenceCapabilitiesLikeCpp::required_like_cpp(
            Arc::clone(&map_corpse_persistence_port),
            Arc::clone(&group_loot_money_persistence_port),
            Arc::clone(&represented_group_persistence_port),
        ),
        wow_world::session::CatalogPersistenceCapabilitiesLikeCpp::required_like_cpp(
            Arc::clone(&quest_poi_persistence_port),
            Arc::clone(&item_template_addon_catalog_persistence_port),
            Arc::clone(&loot_template_catalog_persistence_port),
            Arc::clone(&vendor_catalog_persistence_port),
            Arc::clone(&visibility_spawn_catalog_persistence_port),
            Arc::clone(&gossip_catalog_persistence_port),
        ),
    );
    let session_resources = SessionResources {
        core: SessionCoreCapabilitiesLikeCpp {
            object_mgr_catalogs,
            persistence,
            trainer_store: Arc::clone(&trainer_data_store),
            guid_generator: Arc::clone(&guid_generator),
            item_guid_generator: Arc::clone(&item_guid_generator),
            equipment_set_guid_generator: Arc::clone(&equipment_set_guid_generator),
            void_storage_item_id_generator: Arc::clone(&void_storage_item_id_generator),
            instance_lock_mgr: Arc::clone(&instance_lock_mgr),
        },
        inventory: SessionInventoryCapabilitiesLikeCpp {
            bank_bag_slot_prices_store: Arc::clone(&bank_bag_slot_prices_store),
            currency_types_store: Arc::clone(&currency_types_store),
            import_price_stores: Arc::clone(&import_price_stores),
            emotes_store: Arc::clone(&emotes_store),
            emotes_text_store: Arc::clone(&emotes_text_store),
            item_class_store: Arc::clone(&item_class_store),
            item_currency_cost_store: Arc::clone(&item_currency_cost_store),
            item_extended_cost_store: Arc::clone(&item_extended_cost_store),
            item_store: Arc::clone(&item_store),
            item_child_equipment_store: Arc::clone(&item_child_equipment_store),
            item_appearance_store: Arc::clone(&item_appearance_store),
            item_modified_appearance_store: Arc::clone(&item_modified_appearance_store),
            item_search_name_store: Arc::clone(&item_search_name_store),
            trinity_string_store: Arc::clone(&trinity_string_store),
            heirloom_store: Arc::clone(&heirloom_store),
            toy_store: Arc::clone(&toy_store),
            battle_pet_breed_quality_store: Arc::clone(&battle_pet_breed_quality_store),
            battle_pet_breed_state_store: Arc::clone(&battle_pet_breed_state_store),
            battle_pet_species_store: Arc::clone(&battle_pet_species_entry_store),
            battle_pet_selection_store: Arc::clone(&battle_pet_selection_store),
            battle_pet_species_state_store: Arc::clone(&battle_pet_species_state_store),
            battle_pet_xp_game_table: Arc::clone(&battle_pet_xp_game_table),
            combat_ratings_game_table: Arc::clone(&combat_ratings_game_table),
            shield_block_regular_game_table: Arc::clone(&shield_block_regular_game_table),
            transmog_set_item_store: Arc::clone(&transmog_set_item_store),
            item_price_base_store: Arc::clone(&item_price_base_store),
            item_limit_category_store: Arc::clone(&item_limit_category_store),
            item_limit_category_condition_store: Arc::clone(&item_limit_category_condition_store),
            player_create_info_store: Arc::clone(&player_create_info_store),
            player_create_cast_spell_store: Arc::clone(&player_create_cast_spell_store),
            player_create_custom_spell_store: Arc::clone(&player_create_custom_spell_store),
            player_stats: Arc::clone(&player_stats),
            item_bonus_db2_store: Arc::clone(&item_bonus_db2_store),
            pvp_item_store: Arc::clone(&pvp_item_store),
            item_set_store: Arc::clone(&item_set_store),
            item_set_spell_store: Arc::clone(&item_set_spell_store),
            item_stats_store: Arc::clone(&item_stats_store),
            durability_costs_store: Arc::clone(&durability_costs_store),
            durability_quality_store: Arc::clone(&durability_quality_store),
            item_effect_store: Arc::clone(&item_effect_store),
            item_random_suffix_store: Arc::clone(&item_random_suffix_store),
            item_random_properties_store: Arc::clone(&item_random_properties_store),
            item_spec_override_store: Arc::clone(&item_spec_override_store),
            rand_prop_points_store: Arc::clone(&rand_prop_points_store),
            item_random_enchantment_template_store: Arc::clone(
                &item_random_enchantment_template_store,
            ),
            item_disenchant_loot_store: Arc::clone(&item_disenchant_loot_store),
            loot_stores: Arc::clone(&loot_stores),
        },
        player: SessionPlayerCatalogCapabilitiesLikeCpp {
            condition_store: Arc::clone(&condition_store),
            player_condition_store: Arc::clone(&player_condition_store),
            adventure_map_poi_store: Arc::clone(&adventure_map_poi_store),
            content_tuning_store: Arc::clone(&content_tuning_store),
            curve_store: Arc::clone(&curve_store),
            curve_point_store: Arc::clone(&curve_point_store),
            scaling_stat_distribution_store: Arc::clone(&scaling_stat_distribution_store),
            scaling_stat_values_store: Arc::clone(&scaling_stat_values_store),
            disable_mgr: Arc::clone(&disable_mgr),
            difficulty_store: Arc::clone(&difficulty_store),
            lock_store: Arc::clone(&lock_store),
            spell_item_enchantment_store: Arc::clone(&spell_item_enchantment_store),
            spell_item_enchantment_condition_store: Arc::clone(
                &spell_item_enchantment_condition_store,
            ),
            gem_properties_store: Arc::clone(&gem_properties_store),
            hotfix_blob_cache: Arc::clone(&hotfix_blob_cache),
            tact_key_store: Arc::clone(&tact_key_store),
            skill_store: Arc::clone(&skill_store),
            trait_definition_store: Arc::clone(&trait_definition_store),
            trait_node_entry_store: Arc::clone(&trait_node_entry_store),
            skill_line_store: Arc::clone(&skill_line_store),
            skill_tiers_store: Arc::clone(&skill_tiers_store),
            talent_store: Arc::clone(&talent_store),
            talent_tab_store: Arc::clone(&talent_tab_store),
            num_talents_at_level_store: Arc::clone(&num_talents_at_level_store),
            glyph_properties_store: Arc::clone(&glyph_properties_store),
            chr_races_store: Arc::clone(&chr_races_store),
            chr_classes_store: Arc::clone(&chr_classes_store),
            power_type_store: Arc::clone(&power_type_store),
        },
        spells: SessionSpellCatalogCapabilitiesLikeCpp {
            spell_chain_store: Arc::clone(&spell_chain_store),
            spell_store: Arc::clone(&spell_store),
            spell_acquisition_catalog: Arc::clone(&spell_acquisition_catalog),
            spell_acquisition_safe_cast_spell_ids: Arc::clone(
                &spell_acquisition_safe_cast_spell_ids,
            ),
            spell_acquisition_valid_craft_spell_ids: Arc::clone(
                &spell_acquisition_valid_craft_spell_ids,
            ),
            spell_script_exact_spell_ids: Arc::clone(&spell_script_exact_spell_ids),
            spell_script_all_rank_root_spell_ids: Arc::clone(&spell_script_all_rank_root_spell_ids),
            legacy_spell_script_spell_ids: Arc::clone(&legacy_spell_script_spell_ids),
            spell_linked_rejected_trigger_spell_ids: Arc::clone(
                &spell_linked_rejected_trigger_spell_ids,
            ),
            spell_levels_store: Arc::clone(&spell_levels_store),
            spell_category_store: Arc::clone(&spell_category_store),
            npc_spell_click_store: Arc::clone(&npc_spell_click_store),
            spell_aura_options_store: Arc::clone(&spell_aura_options_store),
            spell_aura_restrictions_store: Arc::clone(&spell_aura_restrictions_store),
            spell_target_restrictions_store: Arc::clone(&spell_target_restrictions_store),
            spell_equipped_items_store: Arc::clone(&spell_equipped_items_store),
            spell_misc_store: Arc::clone(&spell_misc_store),
            spell_group_store: Arc::clone(&spell_group_store),
            spell_group_stack_rule_store: Arc::clone(&spell_group_stack_rule_store),
            spell_linked_store: Arc::clone(&spell_linked_store),
            spell_pet_aura_store: Arc::clone(&spell_pet_aura_store),
            spell_area_store: Arc::clone(&spell_area_store),
            spell_custom_attribute_store: Arc::clone(&spell_custom_attribute_store),
            spell_learn_skill_store: Arc::clone(&spell_learn_skill_store),
            spell_learn_spell_store: Arc::clone(&spell_learn_spell_store),
            spell_proc_store: Arc::clone(&spell_proc_store),
            spell_required_store: Arc::clone(&spell_required_store),
            spell_threat_store: Arc::clone(&spell_threat_store),
            spell_duration_store: Arc::clone(&spell_duration_store),
            spell_radius_store: Arc::clone(&spell_radius_store),
            spell_range_store: Arc::clone(&spell_range_store),
            spell_target_position_store: Arc::clone(&spell_target_position_store),
            movie_store: Arc::clone(&movie_store),
            script_name_interner: Arc::clone(&script_name_interner),
        },
        world: SessionWorldCatalogCapabilitiesLikeCpp {
            area_table_store: Arc::clone(&area_table_store),
            fishing_base_skill_store: Arc::clone(&fishing_base_skill_store),
            area_trigger_db2_store: Arc::clone(&area_trigger_db2_store),
            area_trigger_store: Arc::clone(&area_trigger_store),
            area_trigger_script_store: Arc::clone(&area_trigger_script_store),
            tavern_area_trigger_store: Arc::clone(&tavern_area_trigger_store),
            graveyard_store: Arc::clone(&graveyard_store),
            area_trigger_template_store: Arc::clone(&area_trigger_template_store),
            chr_specialization_store: Arc::clone(&chr_specialization_store),
            dungeon_encounter_store: Arc::clone(&dungeon_encounter_store),
            map_store: Arc::clone(&map_store),
            world_safe_loc_store: Arc::clone(&world_safe_loc_store),
            map_difficulty_store: Arc::clone(&map_difficulty_store),
            map_difficulty_x_condition_store: Arc::clone(&map_difficulty_x_condition_store),
            access_requirement_store: Arc::clone(&access_requirement_store),
            lfg_dungeons_store: Arc::clone(&lfg_dungeons_store),
            lfg_dungeon_store_like_cpp: Arc::clone(&lfg_dungeon_store_like_cpp),
            battlemaster_list_store: Arc::clone(&battlemaster_list_typed_store),
            creature_template_lifecycle_store: Arc::clone(&creature_template_lifecycle_store),
            creature_template_mount_store: Arc::clone(&creature_template_mount_store),
            creature_equipment_store: Arc::clone(&creature_equipment_store),
            creature_display_info_store: Arc::clone(&creature_display_info_store),
            creature_display_info_extra_store: Arc::clone(&creature_display_info_extra_store),
            gameobject_display_info_store: Arc::clone(&gameobject_display_info_store),
            creature_model_info_store: Arc::clone(&creature_model_info_store),
            creature_addon_store: Arc::clone(&creature_addon_store),
            creature_difficulty_store: Arc::clone(&creature_difficulty_store),
            creature_base_stats_store: Arc::clone(&creature_base_stats_store),
            creature_health_rates,
            creature_model_data_store: Arc::clone(&creature_model_data_store),
            mount_store: Arc::clone(&mount_store),
            mount_definition_store: Arc::clone(&mount_definition_store),
            mount_capability_store: Arc::clone(&mount_capability_store),
            mount_type_x_capability_store: Arc::clone(&mount_type_x_capability_store),
            mount_x_display_store: Arc::clone(&mount_x_display_store),
            spell_shapeshift_form_store: Arc::clone(&spell_shapeshift_form_store),
            vehicle_store: Arc::clone(&vehicle_store),
            vehicle_seat_store: Arc::clone(&vehicle_seat_store),
            vehicle_accessory_store: Arc::clone(&vehicle_accessory_store),
            terrain_swap_store: Arc::clone(&terrain_swap_store),
            phase_store: Arc::clone(&phase_store),
            phase_group_store: Arc::clone(&phase_group_store),
        },
        progression: SessionProgressionCapabilitiesLikeCpp {
            quest_store: Arc::clone(&quest_store),
            quest_xp_store: Arc::clone(&quest_xp_store),
            quest_money_reward_store: Arc::clone(&quest_money_reward_store),
            quest_v2_store: Arc::clone(&quest_v2_store),
            quest_info_store: Arc::clone(&quest_info_store),
            quest_package_item_store: Arc::clone(&quest_package_item_store),
            quest_faction_reward_store: Arc::clone(&quest_faction_reward_store),
            progression_faction_store: Arc::clone(&progression_faction_store),
            faction_template_store: Arc::clone(&faction_template_store),
            friendship_rep_reaction_store: Arc::clone(&friendship_rep_reaction_store),
            paragon_reputation_store: Arc::clone(&paragon_reputation_store),
            reputation_reward_rate_store: Arc::clone(&reputation_reward_rate_store),
            creature_onkill_reputation_store: Arc::clone(&creature_onkill_reputation_store),
            reputation_spillover_template_store: Arc::clone(&reputation_spillover_template_store),
            player_xp_table: Arc::clone(&player_xp_table),
            exploration_base_xp_store: Arc::clone(&exploration_base_xp_store),
            exploration_xp_rate: world_config_f32(&world_configs, "RATE_XP_EXPLORE", 1.0),
            // `WorldConfigSet` resolves the external `MaxPlayerLevel` key and indexes
            // the validated value by the matching C++ enum name.
            max_player_level_config: world_config_u32(
                &world_configs,
                "CONFIG_MAX_PLAYER_LEVEL",
                80,
            ),
            max_primary_trade_skills: max_primary_trade_skills_like_cpp(&world_configs),
            is_pvp_realm: is_pvp_realm_type_like_cpp(world_config_u32(
                &world_configs,
                "CONFIG_GAME_TYPE",
                u32::from(REALM_TYPE_NORMAL_LIKE_CPP),
            )),
            is_ffa_pvp_realm: is_ffa_pvp_realm_type_like_cpp(world_config_u32(
                &world_configs,
                "CONFIG_GAME_TYPE",
                u32::from(REALM_TYPE_NORMAL_LIKE_CPP),
            )),
            max_recruit_a_friend_bonus_player_level: world_config_u32(
                &world_configs,
                "CONFIG_MAX_RECRUIT_A_FRIEND_BONUS_PLAYER_LEVEL",
                85,
            ),
            max_recruit_a_friend_bonus_player_level_difference: world_config_u32(
                &world_configs,
                "CONFIG_MAX_RECRUIT_A_FRIEND_BONUS_PLAYER_LEVEL_DIFFERENCE",
                4,
            ),
            rest_offline_wilderness_rate: world_config_f32(
                &world_configs,
                "RATE_REST_OFFLINE_IN_WILDERNESS",
                1.0,
            ),
            rest_offline_tavern_or_city_rate: world_config_f32(
                &world_configs,
                "RATE_REST_OFFLINE_IN_TAVERN_OR_CITY",
                1.0,
            ),
            rest_ingame_rate: world_config_f32(&world_configs, "RATE_REST_INGAME", 1.0),
            min_quest_scaled_xp_ratio: world_config_u32(
                &world_configs,
                "CONFIG_MIN_QUEST_SCALED_XP_RATIO",
                0,
            ),
            min_discovered_scaled_xp_ratio: world_config_u32(
                &world_configs,
                "CONFIG_MIN_DISCOVERED_SCALED_XP_RATIO",
                0,
            ),
        },
        runtime: SessionRuntimePolicyCapabilitiesLikeCpp {
            player_registry: Arc::clone(&player_registry),
            module_registry: Arc::clone(&modules),
            game_event_quest_complete_tx: game_event_quest_complete_tx,
            group_registry: Arc::clone(&group_registry),
            pending_invites: Arc::clone(&pending_invites),
            loot_drop_rates: loot_drop_rates_like_cpp(&world_configs),
            reputation_rates: reputation_rates_like_cpp(&world_configs),
            repair_cost_rate: repair_cost_rate_like_cpp(&world_configs),
            reset_schedule: reset_schedule_like_cpp(&world_configs),
            no_reset_talent_cost: world_config_bool(
                &world_configs,
                "CONFIG_NO_RESET_TALENT_COST",
                false,
            ),
            offhand_check_at_spell_unlearn: world_config_bool(
                &world_configs,
                "CONFIG_OFFHAND_CHECK_AT_SPELL_UNLEARN",
                true,
            ),
            vmap_indoor_check: world_config_bool(&world_configs, "CONFIG_VMAP_INDOOR_CHECK", false),
            start_all_explored: world_config_bool(
                &world_configs,
                "CONFIG_START_ALL_EXPLORED",
                false,
            ),
            start_all_reputation: world_config_bool(&world_configs, "CONFIG_START_ALL_REP", false),
            start_all_spells: world_config_bool(&world_configs, "CONFIG_START_ALL_SPELLS", false),
            support_enabled: world_config_bool(&world_configs, "CONFIG_SUPPORT_ENABLED", true),
            support_tickets_enabled: world_config_bool(
                &world_configs,
                "CONFIG_SUPPORT_TICKETS_ENABLED",
                false,
            ),
            support_bugs_enabled: world_config_bool(
                &world_configs,
                "CONFIG_SUPPORT_BUGS_ENABLED",
                false,
            ),
            support_complaints_enabled: world_config_bool(
                &world_configs,
                "CONFIG_SUPPORT_COMPLAINTS_ENABLED",
                false,
            ),
            support_suggestions_enabled: world_config_bool(
                &world_configs,
                "CONFIG_SUPPORT_SUGGESTIONS_ENABLED",
                false,
            ),
            quest_low_level_hide_diff: world_config_u32(
                &world_configs,
                "CONFIG_QUEST_LOW_LEVEL_HIDE_DIFF",
                4,
            ),
            quest_high_level_hide_diff: world_config_u32(
                &world_configs,
                "CONFIG_QUEST_HIGH_LEVEL_HIDE_DIFF",
                7,
            ),
            enable_ae_loot: world_config_bool(&world_configs, "CONFIG_ENABLE_AE_LOOT", false),
            addon_channel: world_config_bool(&world_configs, "CONFIG_ADDON_CHANNEL", true),
            server_expansion: world_config_u8(&world_configs, "CONFIG_EXPANSION", 2),
            characters_per_realm: world_config_u32(
                &world_configs,
                "CONFIG_CHARACTERS_PER_REALM",
                60,
            ),
            declined_names_used: declined_names_used_like_cpp(
                &world_configs,
                &cfg_categories_store,
            ),
            feature_system_bpay_store_enabled: world_config_bool(
                &world_configs,
                "CONFIG_FEATURE_SYSTEM_BPAY_STORE_ENABLED",
                false,
            ),
            feature_system_character_undelete_enabled: world_config_bool(
                &world_configs,
                "CONFIG_FEATURE_SYSTEM_CHARACTER_UNDELETE_ENABLED",
                false,
            ),
            instance_ignore_raid: world_config_bool(
                &world_configs,
                "CONFIG_INSTANCE_IGNORE_RAID",
                false,
            ),
            instance_ignore_level: world_config_bool(
                &world_configs,
                "CONFIG_INSTANCE_IGNORE_LEVEL",
                false,
            ),
            max_instances_per_hour: world_config_u32(
                &world_configs,
                "CONFIG_MAX_INSTANCES_PER_HOUR",
                5,
            ),
            chat_fake_message_preventing: world_config_bool(
                &world_configs,
                "CONFIG_CHAT_FAKE_MESSAGE_PREVENTING",
                false,
            ),
            party_raid_warnings: world_config_bool(
                &world_configs,
                "CONFIG_CHAT_PARTY_RAID_WARNINGS",
                false,
            ),
            allow_gm_group: world_config_bool(&world_configs, "CONFIG_ALLOW_GM_GROUP", false),
            allow_two_side_interaction_group: world_config_bool(
                &world_configs,
                "CONFIG_ALLOW_TWO_SIDE_INTERACTION_GROUP",
                false,
            ),
            party_level_req: world_config_u32(&world_configs, "CONFIG_PARTY_LEVEL_REQ", 1),
            chat_strict_link_checking_kick: world_config_u8(
                &world_configs,
                "CONFIG_CHAT_STRICT_LINK_CHECKING_KICK",
                0,
            ) != 0,
            chat_level_requirements: ChatLevelRequirementsLikeCpp {
                channel: world_config_u8(&world_configs, "CONFIG_CHAT_CHANNEL_LEVEL_REQ", 1),
                whisper: world_config_u8(&world_configs, "CONFIG_CHAT_WHISPER_LEVEL_REQ", 1),
                emote: world_config_u8(&world_configs, "CONFIG_CHAT_EMOTE_LEVEL_REQ", 1),
                say: world_config_u8(&world_configs, "CONFIG_CHAT_SAY_LEVEL_REQ", 1),
                yell: world_config_u8(&world_configs, "CONFIG_CHAT_YELL_LEVEL_REQ", 1),
            },
            chat_listen_ranges: ChatListenRangesLikeCpp {
                say: world_config_f32(&world_configs, "CONFIG_LISTEN_RANGE_SAY", 25.0),
                text_emote: world_config_f32(&world_configs, "CONFIG_LISTEN_RANGE_TEXTEMOTE", 25.0),
                yell: world_config_f32(&world_configs, "CONFIG_LISTEN_RANGE_YELL", 300.0),
            },
            chat_flood_config: ChatFloodConfigLikeCpp {
                message_count: world_config_u32(
                    &world_configs,
                    "CONFIG_CHATFLOOD_MESSAGE_COUNT",
                    10,
                ),
                message_delay_secs: world_config_u32(
                    &world_configs,
                    "CONFIG_CHATFLOOD_MESSAGE_DELAY",
                    1,
                ),
                addon_message_count: world_config_u32(
                    &world_configs,
                    "CONFIG_CHATFLOOD_ADDON_MESSAGE_COUNT",
                    100,
                ),
                addon_message_delay_secs: world_config_u32(
                    &world_configs,
                    "CONFIG_CHATFLOOD_ADDON_MESSAGE_DELAY",
                    1,
                ),
                mute_time_secs: world_config_u32(&world_configs, "CONFIG_CHATFLOOD_MUTE_TIME", 10),
            },
            packet_spoof_config: PacketSpoofConfigLikeCpp {
                policy: world_config_u32(&world_configs, "CONFIG_PACKET_SPOOF_POLICY", 1),
                ban_mode: world_config_u32(&world_configs, "CONFIG_PACKET_SPOOF_BANMODE", 0),
                ban_duration_secs: world_config_u32(
                    &world_configs,
                    "CONFIG_PACKET_SPOOF_BANDURATION",
                    86_400,
                ),
            },
            player_save_interval_ms: world_config_u32(
                &world_configs,
                "CONFIG_INTERVAL_SAVE",
                15 * 60 * 1000,
            ),
        },
        realm: SessionRealmCapabilitiesLikeCpp {
            realm_id,
            realm_region: active_realm.id.region,
            realm_battlegroup: active_realm.id.site,
            realm_names,
            realm_external_address,
            realm_local_address,
        },
    };
    let session_resources = Arc::new(session_resources);

    // Create SessionManager for ConnectTo flow
    let session_mgr = Arc::new(SessionManager::new());

    // Network configuration
    let bind_ip = wow_config::get_string_default("BindIP", "0.0.0.0");
    let world_port = world_config_u16(&world_configs, "CONFIG_PORT_WORLD", 8085);
    let instance_port = world_config_u16(&world_configs, "CONFIG_PORT_INSTANCE", 8086);
    let max_expansion = world_config_u8(&world_configs, "CONFIG_EXPANSION", 2);
    let mmap_runtime_config = mmap_runtime_config_like_cpp(&world_configs, mmap_disabled_map_ids);
    info!(
        "WORLD: MMap pathfinding: {}, data directory: {}/mmaps",
        if mmap_runtime_config.enabled {
            "enabled"
        } else {
            "disabled"
        },
        mmap_runtime_config.data_dir
    );
    let mmap_pathfinder = mmap_runtime_config.enabled.then(|| {
        Arc::new(
            WorldMMapPathfinderWorkerLikeCpp::spawn_with_parent_map_data_like_cpp(
                &mmap_runtime_config.data_dir,
                map_store.parent_child_map_data_like_cpp(),
            ),
        )
    });

    let realm_addr: SocketAddr = format!("{bind_ip}:{world_port}")
        .parse()
        .context("Invalid bind address")?;
    let instance_addr: SocketAddr = format!("{bind_ip}:{instance_port}")
        .parse()
        .context("Invalid instance bind address")?;

    info!("Starting realm listener on {realm_addr}");
    info!("Starting instance listener on {instance_addr}");

    let mut legacy_creature_aggro_config = legacy_creature_aggro_config_like_cpp(&world_configs);
    legacy_creature_aggro_config.faction_template_store = Some(Arc::clone(&faction_template_store));
    legacy_creature_aggro_config.faction_store = Some(Arc::clone(&progression_faction_store));
    legacy_creature_aggro_config.map_store = Some(Arc::clone(&map_store));
    legacy_creature_aggro_config.disable_mgr = Some(Arc::clone(&disable_mgr));
    legacy_creature_aggro_config.spell_misc_store = Some(Arc::clone(&spell_misc_store));
    legacy_creature_aggro_config.spell_range_store = Some(Arc::clone(&spell_range_store));
    legacy_creature_aggro_config.spell_duration_store = Some(Arc::clone(&spell_duration_store));
    legacy_creature_aggro_config.spell_cooldowns_store = Some(Arc::clone(&spell_cooldowns_store));
    legacy_creature_aggro_config.spell_category_store = Some(Arc::clone(&spell_category_store));
    legacy_creature_aggro_config.spell_x_spell_visual_store =
        Some(Arc::clone(&spell_x_spell_visual_store));
    legacy_creature_aggro_config.spell_target_restrictions_store =
        Some(Arc::clone(&spell_target_restrictions_store));
    legacy_creature_aggro_config.spell_casting_requirements_store =
        Some(Arc::clone(&spell_casting_requirements_store));
    legacy_creature_aggro_config.spell_aura_restrictions_store =
        Some(Arc::clone(&spell_aura_restrictions_store));
    legacy_creature_aggro_config.spell_store = Some(Arc::clone(&spell_store));
    legacy_creature_aggro_config.spell_chain_store = Some(Arc::clone(&spell_chain_store));
    legacy_creature_aggro_config.spell_linked_store = Some(Arc::clone(&spell_linked_store));
    legacy_creature_aggro_config.spell_condition_store = Some(Arc::clone(&condition_store));
    legacy_creature_aggro_config.spell_script_exact_spell_ids_like_cpp =
        Some(Arc::clone(&spell_script_exact_spell_ids));
    legacy_creature_aggro_config.spell_script_all_rank_root_spell_ids_like_cpp =
        Some(Arc::clone(&spell_script_all_rank_root_spell_ids));
    legacy_creature_aggro_config.legacy_spell_script_spell_ids_like_cpp =
        Some(Arc::clone(&legacy_spell_script_spell_ids));
    legacy_creature_aggro_config.spell_linked_rejected_trigger_spell_ids_like_cpp =
        Some(Arc::clone(&spell_linked_rejected_trigger_spell_ids));
    legacy_creature_aggro_config.spell_custom_attribute_store =
        Some(Arc::clone(&spell_custom_attribute_store));
    legacy_creature_aggro_config.difficulty_store = Some(Arc::clone(&difficulty_store));

    let (realm_listener_ready_tx, realm_listener_ready_rx) = tokio::sync::oneshot::channel();
    let (instance_listener_ready_tx, instance_listener_ready_rx) = tokio::sync::oneshot::channel();

    // Spawn realm listener (existing world listener)
    let mut realm_handle = tokio::spawn({
        let lookup = Arc::clone(&account_lookup);
        let listener_policy = world_listener_policy;
        let resources = Arc::clone(&session_resources);
        let mgr = Arc::clone(&session_mgr);
        let smap = Arc::clone(&shared_map);
        let canonical_map = Arc::clone(&canonical_map_manager);
        let spawn_metadata = Arc::clone(&canonical_spawn_metadata);
        let loaded_grid_caches = loaded_grid_creature_respawn_caches.clone();
        let active_sessions = Arc::clone(&active_session_registry);
        let runtime_state = Arc::clone(&world_runtime_state);
        let battle_pet_accounts = Arc::clone(&battle_pet_account_registry);
        let port = instance_port;
        let mmap_config = mmap_runtime_config.clone();
        let mmap_pathfinder = mmap_pathfinder.clone();
        let session_aggro_config = legacy_creature_aggro_config.clone();
        async move {
            wow_network::start_world_listener(
                realm_addr,
                lookup,
                listener_policy,
                move |account, pkt_rx, send_tx, send_write_fence_like_cpp, socket_timeouts| {
                    let resources = Arc::clone(&resources);
                    let mgr = Arc::clone(&mgr);
                    let smap = Arc::clone(&smap);
                    let canonical_map = Arc::clone(&canonical_map);
                    let spawn_metadata = Arc::clone(&spawn_metadata);
                    let loaded_grid_caches = loaded_grid_caches.clone();
                    let active_sessions = Arc::clone(&active_sessions);
                    let runtime_state = Arc::clone(&runtime_state);
                    let mmap_pathfinder = mmap_pathfinder.clone();
                    let session_aggro_config = session_aggro_config.clone();
                    let battle_pet_accounts = Arc::clone(&battle_pet_accounts);
                    create_session(
                        account,
                        pkt_rx,
                        send_tx,
                        send_write_fence_like_cpp,
                        socket_timeouts,
                        resources,
                        mgr,
                        smap,
                        canonical_map,
                        spawn_metadata,
                        loaded_grid_caches,
                        port,
                        max_expansion,
                        mmap_config.clone(),
                        mmap_pathfinder,
                        active_sessions,
                        session_aggro_config,
                        runtime_state,
                        battle_pet_accounts,
                    )
                },
                realm_listener_ready_tx,
            )
            .await
            .context("Realm listener error")
        }
    });

    // Spawn instance listener
    let mut instance_handle = tokio::spawn({
        let mgr = Arc::clone(&session_mgr);
        async move {
            wow_network::start_instance_listener(instance_addr, mgr, instance_listener_ready_tx)
                .await
                .context("Instance listener error")
        }
    });
    let realm_network_abort_handle = realm_handle.abort_handle();
    let instance_network_abort_handle = instance_handle.abort_handle();

    let (realm_listener_ready, instance_listener_ready) =
        tokio::join!(realm_listener_ready_rx, instance_listener_ready_rx);
    let listener_start_error = match (realm_listener_ready, instance_listener_ready) {
        (Ok(Ok(())), Ok(Ok(()))) => None,
        (Ok(Err(error)), _) => Some(format!("realm listener bind failed: {error}")),
        (_, Ok(Err(error))) => Some(format!("instance listener bind failed: {error}")),
        (Err(error), _) => Some(format!("realm listener readiness task failed: {error}")),
        (_, Err(error)) => Some(format!("instance listener readiness task failed: {error}")),
    };
    if let Some(error) = listener_start_error {
        stop_world_network_like_cpp([
            ("realm", &realm_network_abort_handle),
            ("instance", &instance_network_abort_handle),
        ]);
        bail!(error);
    }

    // Match C++ startup: bind both world listeners successfully before
    // clearing the realm's offline flag, but still do so before DB writers and
    // map/runtime producers begin.
    if let Err(error) = set_realm_online(&login_db, realm_id).await {
        stop_world_network_like_cpp([
            ("realm", &realm_network_abort_handle),
            ("instance", &instance_network_abort_handle),
        ]);
        return Err(error);
    }

    let map_update_interval_ms = world_config_u32(&world_configs, "CONFIG_INTERVAL_MAPUPDATE", 10)
        .max(wow_map::MIN_MAP_UPDATE_DELAY_MS);
    let legacy_creature_global_runtime_enabled =
        legacy_creature_global_runtime_enabled_from_config_like_cpp();
    if legacy_creature_global_runtime_enabled {
        info!(
            map_update_interval_ms,
            "RustyCore.LegacyCreatureGlobalRuntime enabled; legacy creature tick owner set to GlobalLegacy"
        );
        match shared_map.write() {
            Ok(mut manager) => {
                manager.set_tick_owner(wow_world::map_manager::RuntimeTickOwner::GlobalLegacy);
            }
            Err(_) => {
                warn!("Legacy MapManager lock poisoned; cannot enable GlobalLegacy tick owner")
            }
        }
    }
    let respawn_condition_interval_ms = world_config_u32(
        &world_configs,
        "CONFIG_RESPAWN_MINCHECKINTERVALMS",
        DEFAULT_RESPAWN_MIN_CHECK_INTERVAL_MS,
    )
    .max(1);
    let respawn_db_mutation_order = Arc::new(Mutex::new(()));
    let respawn_db_producer_stop = Arc::new(AtomicBool::new(false));
    let (respawn_db_writer_tx, mut respawn_db_writer_handle) =
        spawn_respawn_db_writer_like_cpp(Arc::clone(&respawn_persistence));
    let mut map_update_handle = spawn_canonical_map_update_loop(
        Arc::clone(&canonical_map_manager),
        Arc::clone(&shared_map),
        map_update_interval_ms,
        respawn_condition_interval_ms,
        Arc::clone(&canonical_spawn_metadata),
        Arc::clone(&condition_store),
        Arc::clone(&map_store),
        Arc::clone(&game_event_persistence),
        respawn_db_writer_tx.clone(),
        Arc::clone(&respawn_db_mutation_order),
        Arc::clone(&respawn_db_producer_stop),
        loaded_grid_creature_respawn_caches.clone(),
        Arc::clone(&area_trigger_template_store),
        game_event_scheduler,
        Arc::clone(&player_registry),
        Arc::clone(&battlemaster_list_typed_store),
        Arc::clone(&world_state_mgr),
    );
    let mut legacy_creature_runtime_handle = spawn_legacy_creature_runtime_update_loop_like_cpp(
        legacy_creature_global_runtime_enabled,
        Arc::clone(&shared_map),
        Arc::clone(&canonical_map_manager),
        Arc::clone(&map_store),
        mmap_runtime_config.clone(),
        mmap_pathfinder.clone(),
        legacy_creature_aggro_config.clone(),
        map_update_interval_ms,
        Some(respawn_db_writer_tx.clone()),
        Arc::clone(&respawn_db_mutation_order),
        Arc::clone(&respawn_db_producer_stop),
        Some(Arc::clone(&group_registry)),
        Arc::clone(&player_registry),
    );

    let mut ready_check_tick_handle = spawn_group_ready_check_tick_loop(
        Arc::clone(&group_registry),
        Arc::clone(&player_registry),
        map_update_interval_ms,
    );
    let db_keepalive_handle = spawn_db_keepalive_loop_like_cpp(
        Arc::clone(&char_db),
        Arc::clone(&login_db),
        Arc::clone(&world_db),
        db_keepalive_interval_minutes_like_cpp(&world_configs),
    );

    let startup_script_summary = wow_scripts::lifecycle::on_startup().await;
    info!(
        callbacks = startup_script_summary.callbacks,
        "Ran ScriptMgr::OnStartup-style lifecycle hooks"
    );

    let mut map_update_finished = false;
    let mut legacy_creature_runtime_finished = false;
    let mut respawn_db_writer_finished = false;

    // Wait for shutdown signal or a supervised background task failure.
    tokio::select! {
        _ = shutdown_signal() => {
            world_runtime_state.stop_now_like_cpp(SHUTDOWN_EXIT_CODE_LIKE_CPP);
            info!("Shutdown signal received, stopping...");
        }
        result = &mut realm_handle => {
            match result {
                Ok(Ok(())) => {
                    world_runtime_state.stop_now_like_cpp(ERROR_EXIT_CODE_LIKE_CPP);
                    tracing::error!("Realm listener stopped unexpectedly");
                }
                Ok(Err(e)) => {
                    world_runtime_state.stop_now_like_cpp(ERROR_EXIT_CODE_LIKE_CPP);
                    tracing::error!("{e:#}");
                }
                Err(e) => {
                    world_runtime_state.stop_now_like_cpp(ERROR_EXIT_CODE_LIKE_CPP);
                    tracing::error!("Realm listener task failed: {e}");
                }
            }
        }
        result = &mut instance_handle => {
            match result {
                Ok(Ok(())) => {
                    world_runtime_state.stop_now_like_cpp(ERROR_EXIT_CODE_LIKE_CPP);
                    tracing::error!("Instance listener stopped unexpectedly");
                }
                Ok(Err(e)) => {
                    world_runtime_state.stop_now_like_cpp(ERROR_EXIT_CODE_LIKE_CPP);
                    tracing::error!("{e:#}");
                }
                Err(e) => {
                    world_runtime_state.stop_now_like_cpp(ERROR_EXIT_CODE_LIKE_CPP);
                    tracing::error!("Instance listener task failed: {e}");
                }
            }
        }
        result = &mut map_update_handle => {
            map_update_finished = true;
            match result {
                Ok(()) => {
                    world_runtime_state.stop_now_like_cpp(ERROR_EXIT_CODE_LIKE_CPP);
                    tracing::error!("Map update task stopped unexpectedly");
                }
                Err(e) => {
                    world_runtime_state.stop_now_like_cpp(ERROR_EXIT_CODE_LIKE_CPP);
                    tracing::error!("Map update task failed: {e}");
                }
            }
        }
        result = &mut legacy_creature_runtime_handle => {
            legacy_creature_runtime_finished = true;
            match result {
                Ok(()) => {
                    world_runtime_state.stop_now_like_cpp(ERROR_EXIT_CODE_LIKE_CPP);
                    tracing::error!("Legacy creature runtime task stopped unexpectedly");
                }
                Err(e) => {
                    world_runtime_state.stop_now_like_cpp(ERROR_EXIT_CODE_LIKE_CPP);
                    tracing::error!("Legacy creature runtime task failed: {e}");
                }
            }
        }
        result = &mut ready_check_tick_handle => {
            match result {
                Ok(()) => {
                    world_runtime_state.stop_now_like_cpp(ERROR_EXIT_CODE_LIKE_CPP);
                    tracing::error!("Ready-check tick task stopped unexpectedly");
                }
                Err(e) => {
                    world_runtime_state.stop_now_like_cpp(ERROR_EXIT_CODE_LIKE_CPP);
                    tracing::error!("Ready-check tick task failed: {e}");
                }
            }
        }
        result = &mut respawn_db_writer_handle => {
            respawn_db_writer_finished = true;
            world_runtime_state.stop_now_like_cpp(ERROR_EXIT_CODE_LIKE_CPP);
            match result {
                Ok(()) => {
                    tracing::error!("Shared respawn DB writer stopped unexpectedly");
                }
                Err(e) => {
                    tracing::error!("Shared respawn DB writer task failed: {e}");
                }
            }
        }
        result = item_guid_allocator_advisory_lock.wait_until_lost_like_cpp() => {
            world_runtime_state.stop_now_like_cpp(ERROR_EXIT_CODE_LIKE_CPP);
            match result {
                Ok(()) => tracing::error!(
                    "Item GUID allocator advisory-lock monitor stopped unexpectedly"
                ),
                Err(error) => tracing::error!(
                    %error,
                    "Item GUID allocator advisory lock was lost; stopping before another GUID allocation"
                ),
            }
        }
    }

    // Close registration under the same mutex used by `try_register`. An
    // in-flight authenticated connection is therefore either already in the
    // KickAll snapshot or rejected, while C++ KickAll -> UpdateSessions ->
    // StopNetwork ordering remains intact.
    active_session_registry.begin_shutdown_like_cpp();
    let kick_summary = kick_all_sessions_like_cpp(&active_session_registry);
    info!(
        sessions_seen = kick_summary.sessions_seen,
        queued = kick_summary.queued,
        failed = kick_summary.send_failed,
        "Queued World::KickAll-style shutdown kicks"
    );
    if kick_summary.send_failed > 0 {
        world_runtime_state.stop_now_like_cpp(ERROR_EXIT_CODE_LIKE_CPP);
        tracing::error!(
            failed = kick_summary.send_failed,
            "World::KickAll shutdown delivery failed; forcing terminal error status"
        );
    }
    let flush_summary = update_sessions_shutdown_flush_once_like_cpp(
        &active_session_registry,
        1,
        WORLD_SESSION_SHUTDOWN_FLUSH_TIMEOUT_LIKE_CPP,
    )
    .await;
    info!(
        sessions_seen = flush_summary.sessions_seen,
        queued = flush_summary.queued,
        failed = flush_summary.send_failed,
        acked = flush_summary.acked,
        ack_failed = flush_summary.ack_failed,
        ack_timeout = flush_summary.ack_timeout,
        disconnecting = flush_summary.disconnecting,
        "Ran World::UpdateSessions(1)-style shutdown flush"
    );
    if flush_summary.send_failed > 0
        || flush_summary.ack_failed > 0
        || flush_summary.ack_timeout > 0
    {
        world_runtime_state.stop_now_like_cpp(ERROR_EXIT_CODE_LIKE_CPP);
        tracing::error!(
            send_failed = flush_summary.send_failed,
            ack_failed = flush_summary.ack_failed,
            ack_timeout = flush_summary.ack_timeout,
            "World::UpdateSessions shutdown flush was incomplete; forcing terminal error status"
        );
    }
    let network_stop_summary = stop_world_network_like_cpp([
        ("realm", &realm_network_abort_handle),
        ("instance", &instance_network_abort_handle),
    ]);
    info!(
        listeners = network_stop_summary.listeners,
        "Stopped world network listeners like C++ WorldSocketMgr::StopNetwork"
    );
    // Any session that could not receive/ack the explicit commands now sees
    // this cooperative stop at its next update boundary. Registration has
    // already been closed, so the registry can only drain from this point.
    active_session_registry.request_session_stop_like_cpp();
    let sessions_drained = active_session_registry
        .wait_until_empty_like_cpp(WORLD_SESSION_SHUTDOWN_DRAIN_TIMEOUT_LIKE_CPP)
        .await;
    info!(
        drained = sessions_drained,
        remaining = active_session_registry.len_like_cpp(),
        "Waited for task-owned sessions to unregister after shutdown flush"
    );
    if !sessions_drained {
        world_runtime_state.stop_now_like_cpp(ERROR_EXIT_CODE_LIKE_CPP);
        let cancelled = active_session_registry.cancel_all_sessions_like_cpp();
        tracing::error!(
            remaining = active_session_registry.len_like_cpp(),
            cancelled,
            "World-session shutdown grace period expired; force-cancelling registered session futures"
        );
        let forced_sessions_drained = active_session_registry
            .wait_until_empty_like_cpp(WORLD_SESSION_FORCE_CANCEL_TIMEOUT_LIKE_CPP)
            .await;
        if !forced_sessions_drained {
            tracing::error!(
                remaining = active_session_registry.len_like_cpp(),
                timeout_ms = WORLD_SESSION_FORCE_CANCEL_TIMEOUT_LIKE_CPP.as_millis(),
                "Force-cancelled world sessions did not unregister before terminal timeout"
            );
        }
    }

    let battle_pet_operations_drained = battle_pet_account_registry
        .drain_like_cpp(WORLD_SESSION_SHUTDOWN_DRAIN_TIMEOUT_LIKE_CPP)
        .await;
    info!(
        drained = battle_pet_operations_drained,
        "Waited for cancellation-safe battle-pet persistence workers"
    );
    if !battle_pet_operations_drained {
        world_runtime_state.stop_now_like_cpp(ERROR_EXIT_CODE_LIKE_CPP);
        tracing::error!(
            timeout_ms = WORLD_SESSION_SHUTDOWN_DRAIN_TIMEOUT_LIKE_CPP.as_millis(),
            "Battle-pet persistence workers did not drain before the shutdown deadline"
        );
    }

    // Quiesce sessions before closing respawn persistence. Each enabled
    // producer observes this flag on its next interval, performs one final
    // tick to consume state such as `save_respawn_requested`, then returns.
    respawn_db_producer_stop.store(true, Ordering::Release);
    let (map_update_stopped, legacy_runtime_stopped) = tokio::join!(
        stop_respawn_db_producer_like_cpp(
            "canonical-map-update",
            &mut map_update_handle,
            map_update_finished,
        ),
        stop_respawn_db_producer_like_cpp(
            "legacy-creature-runtime",
            &mut legacy_creature_runtime_handle,
            legacy_creature_runtime_finished,
        ),
    );
    if !map_update_stopped || !legacy_runtime_stopped {
        world_runtime_state.stop_now_like_cpp(ERROR_EXIT_CODE_LIKE_CPP);
    }

    // Close the producer-visible mailbox explicitly after both producers have
    // stopped. This makes existing backoff immediately due, rejects any late
    // submission from an aborted blocking tick, and lets the writer drain the
    // latest retained operation for every spawn key.
    respawn_db_writer_tx.close_like_cpp();
    drop(respawn_db_writer_tx);

    // A bounded writer retry failure is explicit and makes shutdown non-zero
    // instead of silently losing acknowledged persistence work. The writer
    // has already been draining concurrently during the session flush.
    if !drain_respawn_db_writer_like_cpp(&mut respawn_db_writer_handle, respawn_db_writer_finished)
        .await
    {
        world_runtime_state.stop_now_like_cpp(ERROR_EXIT_CODE_LIKE_CPP);
    }

    game_event_quest_complete_handle.abort();
    if let Some(db_keepalive_handle) = db_keepalive_handle {
        db_keepalive_handle.abort();
    }
    if let Some(realm_list_update_handle) = realm_list_update_handle {
        realm_list_update_handle.abort();
    }

    if let Err(e) = clear_online_accounts_like_cpp(&login_db, char_db.as_ref(), realm_id).await {
        tracing::error!("Failed to clear online account state for realm {realm_id}: {e}");
    }

    let shutdown_script_summary = wow_scripts::lifecycle::on_shutdown().await;
    info!(
        callbacks = shutdown_script_summary.callbacks,
        "Ran ScriptMgr::OnShutdown-style lifecycle hooks"
    );

    if let Err(e) = set_realm_offline(&login_db, realm_id).await {
        tracing::error!("Failed to mark realm {realm_id} offline: {e}");
    }

    if let Err(error) = item_guid_allocator_advisory_lock.release_like_cpp().await {
        world_runtime_state.stop_now_like_cpp(ERROR_EXIT_CODE_LIKE_CPP);
        tracing::error!(%error, "Failed to release item GUID allocator advisory lock");
    }
    info!(
        exit_code = world_runtime_state.get_exit_code_like_cpp(),
        "World server stopped."
    );
    Ok(process_exit_code_like_cpp(
        world_runtime_state.get_exit_code_like_cpp(),
    ))
}
