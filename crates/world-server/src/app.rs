//! Ordered world-server startup and shutdown composition.
//!
//! The body retains linear C++ `Main.cpp` order and owns no gameplay algorithms.
//! Further splits require typed staged results so resource identity, failure
//! drops, task joins and shutdown order remain explicit.

use super::*;
use wow_database::player::spell_acquisition_adapter::spell_acquisition_port;

#[path = "databases.rs"]
mod databases;

#[path = "guid_generator_startup.rs"]
mod guid_generator_startup;

#[path = "hotfix_delivery_startup.rs"]
mod hotfix_delivery_startup;

use databases::StartupDatabases;
use guid_generator_startup::GuidGeneratorStartup;
use hotfix_delivery_startup::HotfixDeliveryStartup;

#[path = "item_catalogs.rs"]
mod item_catalogs;

use item_catalogs::ItemCatalogs;

#[path = "world_lifecycle_catalogs.rs"]
mod world_lifecycle_catalogs;

use world_lifecycle_catalogs::WorldLifecycleCatalogs;

#[path = "display_vehicle_catalogs.rs"]
mod display_vehicle_catalogs;

use display_vehicle_catalogs::DisplayVehicleCatalogs;

#[path = "skill_trait_catalogs.rs"]
mod skill_trait_catalogs;

use skill_trait_catalogs::SkillTraitCatalogs;

#[path = "spell_core_catalogs.rs"]
mod spell_core_catalogs;

use spell_core_catalogs::SpellCoreCatalogs;

#[path = "spell_metadata_catalogs.rs"]
mod spell_metadata_catalogs;

use spell_metadata_catalogs::SpellMetadataCatalogs;

#[path = "addon_scaling_catalogs.rs"]
mod addon_scaling_catalogs;

use addon_scaling_catalogs::AddonScalingCatalogs;

#[path = "area_trigger_template_catalog.rs"]
mod area_trigger_template_catalog;

#[path = "map_equipment_catalogs.rs"]
mod map_equipment_catalogs;

use map_equipment_catalogs::MapEquipmentCatalogs;

#[path = "spawn_runtime_catalogs.rs"]
mod spawn_runtime_catalogs;

use spawn_runtime_catalogs::SpawnRuntimeCatalogs;

#[path = "mount_catalogs.rs"]
mod mount_catalogs;

use mount_catalogs::MountCatalogs;

#[path = "collection_condition_catalogs.rs"]
mod collection_condition_catalogs;

use collection_condition_catalogs::CollectionConditionCatalogs;

#[path = "battle_pet_stat_catalogs.rs"]
mod battle_pet_stat_catalogs;

use battle_pet_stat_catalogs::BattlePetStatCatalogs;

#[path = "player_creation_catalogs.rs"]
mod player_creation_catalogs;

use player_creation_catalogs::PlayerCreationCatalogs;

#[path = "object_mgr_catalogs.rs"]
mod object_mgr_catalogs;

#[path = "item_enchantment_catalogs.rs"]
mod item_enchantment_catalogs;

use item_enchantment_catalogs::ItemEnchantmentCatalogs;

#[path = "spell_acquisition_catalogs.rs"]
mod spell_acquisition_catalogs;

use spell_acquisition_catalogs::SpellAcquisitionCatalogs;

#[path = "area_trigger_world_catalogs.rs"]
mod area_trigger_world_catalogs;

use area_trigger_world_catalogs::AreaTriggerWorldStartupCatalogs;

#[path = "quest_access_catalogs.rs"]
mod quest_access_catalogs;

use quest_access_catalogs::QuestAccessCatalogs;

#[path = "loot_catalogs.rs"]
mod loot_catalogs;

use loot_catalogs::LootCatalogs;

#[path = "vendor_trainer_catalogs.rs"]
mod vendor_trainer_catalogs;

use vendor_trainer_catalogs::VendorTrainerCatalogs;

#[path = "reserved_game_tele_catalogs.rs"]
mod reserved_game_tele_catalogs;

use reserved_game_tele_catalogs::ReservedGameTeleCatalogs;

#[path = "battle_pet_faction_catalogs.rs"]
mod battle_pet_faction_catalogs;

use battle_pet_faction_catalogs::BattlePetFactionCatalogs;

#[path = "progression_reward_catalogs.rs"]
mod progression_reward_catalogs;

use progression_reward_catalogs::ProgressionRewardCatalogs;

#[path = "player_choice_catalogs.rs"]
mod player_choice_catalogs;

#[path = "spell_visual_jump_catalogs.rs"]
mod spell_visual_jump_catalogs;

use spell_visual_jump_catalogs::SpellVisualJumpCatalogs;

#[path = "reputation_catalogs.rs"]
mod reputation_catalogs;

use reputation_catalogs::ReputationCatalogs;

#[path = "condition_catalogs.rs"]
mod condition_catalogs;

#[path = "spell_world_startup_catalogs.rs"]
mod spell_world_startup_catalogs;

use spell_world_startup_catalogs::SpellWorldStartupCatalogs;

#[path = "group_instance_startup.rs"]
mod group_instance_startup;

#[path = "game_event_startup.rs"]
mod game_event_startup;

#[path = "session_persistence_startup.rs"]
mod session_persistence_startup;

#[path = "session_runtime_policy.rs"]
mod session_runtime_policy;

#[path = "session_progression_policy.rs"]
mod session_progression_policy;

#[path = "session_handler_policy_catalogs.rs"]
mod session_handler_policy_catalogs;

#[path = "session_player_policy_catalogs.rs"]
mod session_player_policy_catalogs;

#[path = "world_listener_startup.rs"]
mod world_listener_startup;

#[path = "runtime_network_configuration.rs"]
mod runtime_network_configuration;

use group_instance_startup::GroupInstanceStartup;
use runtime_network_configuration::RuntimeNetworkConfiguration;
use world_listener_startup::WorldListenerStartup;

#[path = "entry.rs"]
mod entry;
pub use entry::{run, run_with_composition, run_with_modules};

async fn run_inner(
    args: Vec<String>,
    modules: Arc<wow_module_api::ModuleRegistry>,
    module_lock: &'static str,
) -> Result<ExitCode> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();
    wow_logging::install_panic_hook_like_cpp();
    // A panic inside a linked-module callback is caught at the module boundary
    // and only disables that module (#1445): log it as contained, not fatal.
    wow_logging::set_contained_panic_probe_like_cpp(wow_module_api::module_callback_in_progress);

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
    let StartupDatabases {
        login_db,
        char_db,
        world_db,
        hotfix_db,
        world_reference_catalog_persistence,
        world_auxiliary_catalog_persistence,
        world_object_catalog_persistence,
        quest_catalog_persistence,
        gameplay_rule_catalog_persistence,
        condition_disable_catalog_persistence,
        player_base_stats_persistence,
        player_creation_catalog_persistence,
        skill_world_rules_persistence,
    } = databases::initialize_databases(module_lock).await?;

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
    let world_module_hooks = module_hooks::WorldModuleHooksLikeCpp::after_config_load_like_cpp(
        Arc::clone(&modules),
        &world_configs,
        realm_id,
    );
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

    let GuidGeneratorStartup {
        guid_generator,
        mut item_guid_allocator_advisory_lock,
        item_guid_generator,
        equipment_set_guid_generator,
        void_storage_item_id_generator,
    } = guid_generator_startup::initialize_guid_generators(&char_db).await?;

    let char_db = Arc::new(char_db);

    let ItemCatalogs {
        data_dir,
        locale,
        currency_types_store,
        import_price_stores,
        bank_bag_slot_prices_store,
        item_class_store,
        item_currency_cost_store,
        item_extended_cost_store,
        item_store,
        item_child_equipment_store,
        item_price_base_store,
        item_limit_category_store,
        item_limit_category_condition_store,
        item_bonus_db2_store,
        pvp_item_store,
        item_set_store,
        item_set_spell_store,
    } = item_catalogs::load_item_catalogs()?;
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
        hotfix::chr_specialization::load_chr_specialization_store_like_cpp(
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
        crate::world::reference_catalog::load_world_safe_locs_like_cpp(
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
    let (phase_store, phase_group_store) = catalogs::phase_hotfix::load_phase_stores_like_cpp(
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
        catalogs::phase_world::load_phase_world_catalogs_like_cpp(
            &phase_world_adapter,
            &area_table_store,
            &phase_store,
            &map_store,
            |phase_id| ui_map_x_map_art_store.is_ui_map_phase(phase_id),
        )
        .await?;
    let _phase_name_store = Arc::new(phase_name_store);
    let terrain_swap_store = Arc::new(terrain_swap_store);
    let WorldLifecycleCatalogs {
        mut graveyard_store,
        mut gossip_store,
        gossip_catalog_adapter,
        spawn_group_store,
        creature_template_store,
        gameobject_template_store,
        creature_template_classification_store,
        mut creature_template_lifecycle_store,
        creature_template_sparring_store,
        gameobject_template_lifecycle_store,
        gameobject_override_lifecycle_store,
        mut script_name_interner,
        _scene_template_store,
    } = world_lifecycle_catalogs::load_world_lifecycle_catalogs(
        &world_auxiliary_catalog_persistence,
        &world_reference_catalog_persistence,
        &world_object_catalog_persistence,
        &world_db,
        world_safe_loc_store.as_ref(),
        area_table_store.as_ref(),
    )
    .await?;
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
        hotfix::difficulty::load_difficulty_store_like_cpp(
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
        crate::world::object_catalog::load_creature_difficulties_like_cpp(
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
        crate::world::object_catalog::load_creature_base_stats_like_cpp(
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
        crate::world::object_catalog::load_creature_mounts_like_cpp(
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
        hotfix::creature_display::load_creature_display_info_store_like_cpp(
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
    let DisplayVehicleCatalogs {
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
    } = display_vehicle_catalogs::load_display_vehicle_catalogs(
        &data_dir,
        &locale,
        &hotfix_db,
        &world_db,
        &creature_display_hotfix_persistence,
        creature_display_info_store.as_ref(),
        &world_object_catalog_persistence,
        &world_reference_catalog_persistence,
    )
    .await?;
    let SkillTraitCatalogs {
        skill_line_store,
        skill_store,
        trait_tree_skill_line_index,
        trait_definition_store,
        trait_definition_locale_store,
        trait_currency_source_locale_store,
        trait_node_entry_store,
        skill_tiers_store,
        talent_store,
        talent_tab_store,
        num_talents_at_level_store,
        glyph_properties_store,
    } = skill_trait_catalogs::load_skill_trait_catalogs(
        &data_dir,
        &locale,
        &hotfix_db,
        &db2_hotfix_removals,
        &skill_world_rules_persistence,
    )
    .await?;
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
    let chr_customization_display_info_store = Arc::new(
        wow_data::character_progression::ChrCustomizationDisplayInfoStore::load(&data_dir, &locale)
            .context("Failed to load ChrCustomizationDisplayInfo.db2")?,
    );
    let shapeshift_display_links_absent =
        crate::static_data_overlay::shapeshift_display_links_absent(
            &chr_customization_display_info_store,
            &static_data_overlay_persistence,
        )
        .await
        .context("Failed to certify effective shapeshift customization display links")?;
    info!(
        "Loaded {} ChrCustomizationDisplayInfo DB2 rows; shapeshift fallback link gate: {}",
        chr_customization_display_info_store.len(),
        shapeshift_display_links_absent
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
    let SpellCoreCatalogs {
        spell_core_hotfix_persistence,
        spell_levels_store,
        spell_acquisition_startup_persistence,
        spell_name_store,
        mut spell_store,
    } = spell_core_catalogs::load_spell_core_catalogs(
        &data_dir,
        &locale,
        &hotfix_db,
        &world_db,
        &db2_hotfix_removals,
        creature_family_store.as_ref(),
        skill_store.as_ref(),
        creature_template_lifecycle_store.as_ref(),
    )
    .await?;
    let SpellMetadataCatalogs {
        spell_category_store,
        spell_aura_options_store,
        spell_aura_restrictions_store,
        spell_casting_requirements_store,
        spell_class_options_store,
        spell_label_store,
        spell_equipped_items_store,
        spell_target_restrictions_store,
        spell_misc_store,
        spell_procs_per_minute_store,
        spell_duration_store,
        spell_cooldowns_store,
    } = spell_metadata_catalogs::load_spell_metadata_catalogs(
        &data_dir,
        &locale,
        &db2_hotfix_removals,
        &spell_core_hotfix_persistence,
        creature_family_store.as_ref(),
        skill_store.as_ref(),
        spell_levels_store.as_ref(),
        &spell_store,
    )
    .await?;
    let AddonScalingCatalogs {
        spell_shapeshift_form_store,
        creature_addon_store,
        active_event_store,
        world_state_store,
        trainer_store,
        curve_store,
        curve_point_store,
        scaling_stat_distribution_store,
        scaling_stat_values_store,
    } = addon_scaling_catalogs::load_addon_scaling_catalogs(
        &data_dir,
        &locale,
        &world_object_catalog_persistence,
        &world_reference_catalog_persistence,
        creature_template_lifecycle_store.as_ref(),
        creature_spawn_store.as_ref(),
        creature_display_info_store.as_ref(),
        emotes_store.as_ref(),
        anim_kit_store.as_ref(),
        &spell_store,
        spell_misc_store.as_ref(),
        spell_duration_store.as_ref(),
    )
    .await?;
    let area_trigger_template_store =
        area_trigger_template_catalog::load_area_trigger_template_catalog(
            &world_db,
            world_safe_loc_store.as_ref(),
            curve_store.as_ref(),
            &mut script_name_interner,
        )
        .await?;
    let MapEquipmentCatalogs {
        map_difficulty_store,
        map_difficulty_x_condition_store,
        lfg_dungeons_store,
        item_appearance_store,
        item_modified_appearance_store,
        item_stats_store,
        creature_equipment_store,
    } = map_equipment_catalogs::load_map_equipment_catalogs(
        &data_dir,
        &locale,
        &hotfix_db,
        &world_object_catalog_persistence,
        creature_template_lifecycle_store.as_ref(),
    )
    .await?;

    let SpawnRuntimeCatalogs {
        mut script_name_interner,
        game_event_persistence,
        canonical_spawn_metadata,
        persisted_respawn_times,
        respawn_persistence,
        world_state_mgr,
    } = spawn_runtime_catalogs::load_spawn_runtime_catalogs(
        &char_db,
        &world_db,
        map_store.as_ref(),
        map_difficulty_store.as_ref(),
        &spawn_group_store,
        creature_equipment_store.as_ref(),
        area_trigger_template_store.as_ref(),
        &spell_store,
        script_name_interner,
        area_table_store.as_ref(),
    )
    .await?;

    let MountCatalogs {
        mount_store,
        mount_definition_store,
        mount_capability_store,
        mount_type_x_capability_store,
        mount_x_display_store,
    } = mount_catalogs::load_mount_catalogs(&data_dir, &locale, &hotfix_db, &world_db).await?;
    let CollectionConditionCatalogs {
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
    } = collection_condition_catalogs::load_collection_condition_catalogs(
        &data_dir,
        &locale,
        &hotfix_db,
        &db2_hotfix_removals,
        &world_reference_catalog_persistence,
    )
    .await?;

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
        crate::world::auxiliary_catalog::load_trinity_strings_like_cpp(
            &world_auxiliary_catalog_persistence,
        )
        .await
        .context("Failed to load C++ trinity_string rows")?,
    );
    info!(
        "Loaded {} C++ trinity_string rows",
        trinity_string_store.len()
    );

    let BattlePetStatCatalogs {
        battle_pet_breed_quality_store,
        battle_pet_breed_state_store,
        battle_pet_species_entry_store,
        battle_pet_species_state_store,
        battle_pet_xp_game_table,
        combat_ratings_game_table,
        shield_block_regular_game_table,
    } = battle_pet_stat_catalogs::load_battle_pet_stat_catalogs(&data_dir, &locale)?;

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

    let PlayerCreationCatalogs {
        player_create_info_store,
        player_stats,
        player_create_cast_spell_store,
        player_create_custom_spell_store,
        starting_items,
    } = player_creation_catalogs::load_player_creation_catalogs(
        &data_dir,
        &locale,
        &world_db,
        &player_creation_catalog_persistence,
        &player_base_stats_persistence,
        &map_store,
        &chr_races_store,
        &chr_classes_store,
        &chr_model_store,
        &chr_race_x_chr_model_store,
        &gameobject_template_lifecycle_store,
        world_config_u8(&world_configs, "CONFIG_MAX_PLAYER_LEVEL", 80),
    )
    .await?;

    let object_mgr_catalogs = object_mgr_catalogs::load_object_mgr_catalogs(
        &world_db,
        &gameobject_template_lifecycle_store,
        &creature_template_lifecycle_store,
        &item_stats_store,
    )
    .await?;

    let ItemEnchantmentCatalogs {
        durability_costs_store,
        durability_quality_store,
        item_effect_store,
        lock_store,
        item_random_suffix_store,
        item_random_properties_store,
        item_spec_override_store,
        rand_prop_points_store,
        item_disenchant_loot_store,
        item_random_enchantment_template_store,
        spell_item_enchantment_store,
        spell_item_enchantment_condition_store,
        gem_properties_store,
        _spell_enchant_proc_outcome,
    } = item_enchantment_catalogs::load_item_enchantment_catalogs(
        &data_dir,
        &locale,
        &world_db,
        &static_data_overlay_persistence,
    )
    .await?;

    let HotfixDeliveryStartup {
        hotfix_blob_cache,
        tact_key_store,
    } = hotfix_delivery_startup::load_hotfix_delivery_startup(
        &data_dir,
        &locale,
        &hotfix_delivery_metadata_persistence,
    )
    .await?;

    let SpellAcquisitionCatalogs {
        spell_radius_store,
        spell_range_store,
        serverside_spell_store,
        spell_acquisition_catalog,
        spell_chain_store,
        spell_learn_skill_store,
        spell_learn_spell_store,
        spell_custom_attribute_store,
    } = spell_acquisition_catalogs::load_spell_acquisition_catalogs(
        &data_dir,
        &locale,
        &spell_core_hotfix_persistence,
        &db2_hotfix_removals,
        &spell_acquisition_startup_persistence,
        difficulty_store.as_ref(),
        skill_store.as_ref(),
        &spell_name_store,
        &mut spell_store,
    )
    .await?;

    let AreaTriggerWorldStartupCatalogs {
        area_trigger_store,
        area_trigger_script_store,
        tavern_area_trigger_store,
    } = area_trigger_world_catalogs::load_area_trigger_world_startup_catalogs(
        &world_db,
        area_trigger_db2_store.as_ref(),
        &mut script_name_interner,
    )
    .await?;

    let QuestAccessCatalogs {
        quest_store,
        lfg_dungeon_store_like_cpp,
        spell_area_store,
        spell_world_catalog_persistence,
        access_requirement_store,
        disable_mgr,
    } = quest_access_catalogs::load_quest_access_catalogs(
        &quest_catalog_persistence,
        &world_db,
        lfg_dungeons_store.as_ref(),
        map_difficulty_store.as_ref(),
        &spell_store,
        area_table_store.as_ref(),
        &world_auxiliary_catalog_persistence,
        &map_store,
        &item_store,
        achievement_store.as_ref(),
        &condition_disable_catalog_persistence,
        criteria_store.as_ref(),
        battlemaster_list_store.as_ref(),
    )
    .await?;
    let mmap_disabled_map_ids = disable_mgr.disabled_mmap_map_ids_like_cpp();
    info!(
        "Loaded {} C++ mmap disable rows",
        mmap_disabled_map_ids.len()
    );

    let LootCatalogs {
        loot_stores,
        _gameobject_for_quest_store,
    } = loot_catalogs::load_loot_catalogs(
        &world_db,
        &item_store,
        gameobject_template_lifecycle_store.as_ref(),
    )
    .await?;
    let ReservedGameTeleCatalogs {
        _reserved_name_store,
        _game_tele_store,
    } = reserved_game_tele_catalogs::load_reserved_game_tele_catalogs(&char_db, &world_db).await?;
    let VendorTrainerCatalogs {
        _npc_vendor_store,
        trainer_data_store,
    } = vendor_trainer_catalogs::load_vendor_trainer_catalogs(
        &gameplay_rule_catalog_persistence,
        &world_db,
        &spell_store,
        serverside_spell_store.as_ref(),
        difficulty_store.as_ref(),
        skill_line_store.as_ref(),
        creature_template_store.as_ref(),
        &gossip_store,
    )
    .await?;

    let BattlePetFactionCatalogs {
        battle_pet_selection_store,
        _faction_change_store,
    } = battle_pet_faction_catalogs::load_battle_pet_faction_catalogs(
        &world_db,
        battle_pet_species_entry_store.as_ref(),
        &gameplay_rule_catalog_persistence,
        achievement_store.as_ref(),
        quest_store.as_ref(),
        faction_store.as_ref(),
        &spell_store,
        char_titles_store.as_ref(),
        item_stats_store.as_ref(),
    )
    .await?;

    let ProgressionRewardCatalogs {
        player_xp_table,
        exploration_base_xp_store,
        quest_xp_store,
        quest_money_reward_store,
        quest_v2_store,
        quest_info_store,
        quest_package_item_store,
    } = progression_reward_catalogs::load_progression_reward_catalogs(
        &world_db, &data_dir, &locale,
    )
    .await?;
    let _player_choice_store = player_choice_catalogs::load_player_choice_store(
        &world_db,
        char_titles_store.as_ref(),
        quest_package_item_store.as_ref(),
        skill_line_store.as_ref(),
        item_stats_store.as_ref(),
        currency_types_store.as_ref(),
        faction_store.as_ref(),
    )
    .await?;
    let SpellVisualJumpCatalogs {
        _spell_visual_store,
        spell_x_spell_visual_store,
        _jump_charge_params_store,
    } = spell_visual_jump_catalogs::load_spell_visual_jump_catalogs(
        &data_dir,
        &locale,
        &spell_core_hotfix_persistence,
        &db2_hotfix_removals,
        &world_db,
        curve_store.as_ref(),
    )
    .await?;
    let ReputationCatalogs {
        quest_faction_reward_store,
        progression_faction_store,
        faction_template_store,
        friendship_rep_reaction_store,
        paragon_reputation_store,
        reputation_reward_rate_store,
        creature_onkill_reputation_store,
        reputation_spillover_template_store,
    } = reputation_catalogs::load_reputation_catalogs(
        &data_dir,
        &locale,
        &world_db,
        creature_template_lifecycle_store.as_ref(),
    )
    .await?;

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
        Arc::clone(&battle_pet_xp_game_table),
        realm_id,
        active_realm.id.address_like_cpp(),
    ));

    // Build the process-wide packet-handler registry before admitting sessions.
    let packet_handler_registry = crate::compose_packet_handlers_like_cpp()?;
    info!("Loaded {} packet handlers", packet_handler_registry.len());

    // Build account lookup
    let account_lookup: Arc<dyn AccountLookup> = Arc::new(DbAccountLookup {
        login_db: Arc::clone(&login_db),
        realm_id,
        win64_auth_seed,
    });

    let player_registry = Arc::new(PlayerRegistry::new());
    let active_session_registry = Arc::new(ActiveWorldSessionRegistryLikeCpp::new());
    let condition_store = condition_catalogs::load_condition_store(
        &condition_disable_catalog_persistence,
        loot_stores.as_ref(),
        item_store.as_ref(),
        &mut spell_store,
        area_table_store.as_ref(),
        skill_line_store.as_ref(),
        map_store.as_ref(),
        phase_store.as_ref(),
        quest_store.as_ref(),
        area_trigger_db2_store.as_ref(),
        &mut graveyard_store,
        &spawn_group_store,
        creature_template_store.as_ref(),
        gameobject_template_store.as_ref(),
        trainer_store.as_ref(),
        conversation_line_template_store.as_ref(),
        area_trigger_template_store.as_ref(),
        creature_spawn_store.as_ref(),
        gameobject_spawn_store.as_ref(),
        active_event_store.as_ref(),
        world_state_store.as_ref(),
        difficulty_store.as_ref(),
        faction_store.as_ref(),
        achievement_store.as_ref(),
        char_titles_store.as_ref(),
        battle_pet_species_store.as_ref(),
        scenario_step_store.as_ref(),
        scene_script_package_store.as_ref(),
        player_condition_store.as_ref(),
        max_skill_value_like_cpp(&world_configs),
        &mut gossip_store,
        &mut phase_info_store,
    )
    .await?;
    wow_world::conditions::set_condition_mgr_store_like_cpp(Arc::clone(&condition_store));
    let graveyard_store = Arc::new(graveyard_store);
    let SpellWorldStartupCatalogs {
        npc_spell_click_store,
        spell_target_position_store,
        spell_proc_store,
        spell_required_store,
        spell_group_store,
        spell_group_stack_rule_store,
        spell_threat_store,
        spell_linked_rejected_trigger_spell_ids,
        spell_linked_store,
        spell_pet_aura_store,
        spell_acquisition_safe_cast_spell_ids,
        spell_acquisition_valid_craft_spell_ids,
        spell_script_exact_spell_ids,
        spell_script_all_rank_root_spell_ids,
        legacy_spell_script_spell_ids,
    } = spell_world_startup_catalogs::load_spell_world_startup_catalogs(
        &gameplay_rule_catalog_persistence,
        &mut creature_template_lifecycle_store,
        &spell_world_catalog_persistence,
        map_store.as_ref(),
        &mut spell_store,
        serverside_spell_store.as_ref(),
        spell_chain_store.as_ref(),
        difficulty_store.as_ref(),
        spell_aura_options_store.as_ref(),
        spell_class_options_store.as_ref(),
        spell_procs_per_minute_store.as_ref(),
        chr_races_store.as_ref(),
        creature_display_info_store.as_ref(),
        &spell_acquisition_startup_persistence,
        &db2_hotfix_removals,
        spell_acquisition_catalog.as_ref(),
        spell_aura_restrictions_store.as_ref(),
        spell_casting_requirements_store.as_ref(),
        spell_equipped_items_store.as_ref(),
        spell_area_store.as_ref(),
        item_stats_store.as_ref(),
        &data_dir,
        &locale,
    )
    .await?;
    let spell_store = Arc::new(spell_store);

    let GroupInstanceStartup {
        group_registry,
        pending_invites,
        represented_group_persistence_adapter,
        live_terrain,
        instance_lock_persistence_port,
        registered_instance_ids,
        instance_lock_mgr,
    } = group_instance_startup::load_group_instance_startup(
        &char_db,
        difficulty_store.as_ref(),
        map_store.as_ref(),
        map_difficulty_store.as_ref(),
        &data_dir,
    )
    .await?;

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
    register_loaded_instance_ids(canonical_map_manager.as_ref(), &registered_instance_ids);

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
        spell_x_spell_visual_store: Arc::clone(&spell_x_spell_visual_store),
        vehicle_store: Arc::clone(&vehicle_store),
        vehicle_seat_store: Arc::clone(&vehicle_seat_store),
        vehicle_accessory_store: Arc::clone(&vehicle_accessory_store),
        gameobject_template_store: Arc::clone(&gameobject_template_lifecycle_store),
        gameobject_override_store: Arc::clone(&gameobject_override_lifecycle_store),
    };
    let game_event_scheduler = game_event_startup::start_game_event_system(
        &canonical_spawn_metadata,
        game_event_persistence.as_ref(),
        &canonical_map_manager,
        &loaded_grid_creature_respawn_caches,
        battlemaster_list_typed_store.as_ref(),
        player_registry.as_ref(),
        &world_state_mgr,
    )
    .await?;

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

    let persistence = session_persistence_startup::build_session_persistence_startup(
        &char_db,
        &login_db,
        &world_db,
        &gossip_catalog_adapter,
        represented_group_persistence_adapter,
        &instance_lock_persistence_port,
    )
    .await?;
    let grid_canonical_map_manager = Arc::clone(&canonical_map_manager);
    let grid_spawn_metadata = Arc::clone(&canonical_spawn_metadata);
    let grid_loaded_caches = loaded_grid_creature_respawn_caches.clone();
    let grid_map_store = Arc::clone(&map_store);
    let grid_area_trigger_template_store = Arc::clone(&area_trigger_template_store);
    let player_grid_loader = Arc::new(move |map_id, instance_id, position| {
        ensure_login_player_grid_loaded_like_cpp(
            &grid_canonical_map_manager,
            &grid_spawn_metadata,
            &grid_loaded_caches,
            grid_area_trigger_template_store.as_ref(),
            Some(grid_map_store.as_ref()),
            map_id,
            instance_id,
            position,
        )
    });
    let session_resources = SessionResources {
        core: SessionCoreCapabilitiesLikeCpp {
            packet_handlers: Arc::clone(&packet_handler_registry),
            handler_catalogs: Arc::new(wow_world::session::SessionHandlerCatalogsLikeCpp {
                object_mgr: Arc::clone(&object_mgr_catalogs),
                player_grid_loader,
                area_triggers: Arc::new(wow_world::session::AreaTriggerCatalogsLikeCpp {
                    db2: Arc::clone(&area_trigger_db2_store),
                    destinations: Arc::clone(&area_trigger_store),
                    scripts: Arc::clone(&area_trigger_script_store),
                    taverns: Arc::clone(&tavern_area_trigger_store),
                    script_dispatcher: None,
                }),
                item_valuation: Arc::new(wow_world::session::ItemValuationCatalogsLikeCpp {
                    import_prices: Arc::clone(&import_price_stores),
                    price_base: Arc::clone(&item_price_base_store),
                    item_classes: Arc::clone(&item_class_store),
                    currency_costs: Arc::clone(&item_currency_cost_store),
                    disenchant_loot: Arc::clone(&item_disenchant_loot_store),
                }),
                player_bootstrap: session_player_policy_catalogs::build_player_bootstrap(
                    &player_create_info_store,
                    &glyph_properties_store,
                    &talent_tab_store,
                    &trait_node_entry_store,
                    &player_create_cast_spell_store,
                    &player_create_custom_spell_store,
                    &world_configs,
                ),
                player_rest_rates: session_player_policy_catalogs::build_player_rest_rates(
                    &world_configs,
                ),
                creature_spawns: Arc::new(wow_world::session::CreatureSpawnCatalogsLikeCpp {
                    difficulty: Arc::clone(&creature_difficulty_store),
                    base_stats: Arc::clone(&creature_base_stats_store),
                    health_rates: creature_health_rates,
                    addons: Arc::clone(&creature_addon_store),
                    equipment: Arc::clone(&creature_equipment_store),
                    power_types: Arc::clone(&power_type_store),
                    localized_names: Arc::clone(&object_mgr_catalogs.creature),
                }),
                progression: session_player_policy_catalogs::build_progression(
                    &player_xp_table,
                    &exploration_base_xp_store,
                    &world_configs,
                ),
                battle_pet_trainer_selection: Arc::clone(&battle_pet_selection_store),
                quest_info: Arc::clone(&quest_info_store),
                chat_policy: session_handler_policy_catalogs::build_chat_policy(&world_configs),
                group_invite_policy: session_handler_policy_catalogs::build_group_invite_policy(
                    &world_configs,
                ),
                trade_policy: session_handler_policy_catalogs::build_trade_policy(&world_configs),
                support_feature_policy:
                    session_handler_policy_catalogs::build_support_feature_policy(
                        &world_configs,
                        &cfg_categories_store,
                    ),
                player_regeneration_rates:
                    session_player_policy_catalogs::build_player_regeneration_rates(&world_configs),
                bank_bag_slot_prices: Arc::clone(&bank_bag_slot_prices_store),
                adventure_map_pois: Arc::clone(&adventure_map_poi_store),
                battlemaster_lists: Arc::clone(&battlemaster_list_typed_store),
                emotes: Arc::clone(&emotes_store),
                emotes_text: Arc::clone(&emotes_text_store),
                graveyards: Arc::clone(&graveyard_store),
                lfg_dungeons: Arc::clone(&lfg_dungeon_store_like_cpp),
                tact_keys: Arc::clone(&tact_key_store),
                hotfixes: Arc::clone(&hotfix_blob_cache),
                modules: Arc::clone(&modules),
                id_generators: Arc::new(wow_world::session::SessionIdGeneratorsLikeCpp {
                    player: Arc::clone(&guid_generator),
                    item: Arc::clone(&item_guid_generator),
                    equipment_set: Arc::clone(&equipment_set_guid_generator),
                    void_storage_item: Arc::clone(&void_storage_item_id_generator),
                }),
            }),
            gameobject_template_lifecycle_store: Arc::clone(&gameobject_template_lifecycle_store),
            persistence,
            trainer_store: Arc::clone(&trainer_data_store),
            instance_lock_mgr: Arc::clone(&instance_lock_mgr),
        },
        inventory: SessionInventoryCapabilitiesLikeCpp {
            currency_types_store: Arc::clone(&currency_types_store),
            item_extended_cost_store: Arc::clone(&item_extended_cost_store),
            item_store: Arc::clone(&item_store),
            item_child_equipment_store: Arc::clone(&item_child_equipment_store),
            item_appearance_store: Arc::clone(&item_appearance_store),
            item_modified_appearance_store: Arc::clone(&item_modified_appearance_store),
            item_search_name_store: Arc::clone(&item_search_name_store),
            trinity_string_store: Arc::clone(&trinity_string_store),
            heirloom_store: Arc::clone(&heirloom_store),
            toy_store: Arc::clone(&toy_store),
            combat_ratings_game_table: Arc::clone(&combat_ratings_game_table),
            regen_game_tables: crate::session_resources::load_regen_game_tables(&data_dir)?,
            shield_block_regular_game_table: Arc::clone(&shield_block_regular_game_table),
            transmog_set_item_store: Arc::clone(&transmog_set_item_store),
            item_limit_category_store: Arc::clone(&item_limit_category_store),
            item_limit_category_condition_store: Arc::clone(&item_limit_category_condition_store),
            player_stats: Arc::clone(&player_stats),
            item_bonus_db2_store: Arc::clone(&item_bonus_db2_store),
            pvp_item_store: Arc::clone(&pvp_item_store),
            item_set_store: Arc::clone(&item_set_store),
            item_set_spell_store: Arc::clone(&item_set_spell_store),
            item_stats_store: Arc::clone(&item_stats_store),
            starting_items,
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
            loot_stores: Arc::clone(&loot_stores),
        },
        player: SessionPlayerCatalogCapabilitiesLikeCpp {
            condition_store: Arc::clone(&condition_store),
            player_condition_store: Arc::clone(&player_condition_store),
            condition_presence_catalog: Arc::clone(&condition_presence_catalog),
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
            skill_store: Arc::clone(&skill_store),
            trait_definition_store: Arc::clone(&trait_definition_store),
            trait_definition_locale_store: Arc::clone(&trait_definition_locale_store),
            trait_currency_source_locale_store: Arc::clone(&trait_currency_source_locale_store),
            trait_tree_skill_line_index: Arc::clone(&trait_tree_skill_line_index),
            skill_line_store: Arc::clone(&skill_line_store),
            skill_tiers_store: Arc::clone(&skill_tiers_store),
            talent_store: Arc::clone(&talent_store),
            num_talents_at_level_store: Arc::clone(&num_talents_at_level_store),
            chr_races_store: Arc::clone(&chr_races_store),
            chr_classes_store: Arc::clone(&chr_classes_store),
            shapeshift_display_links_absent,
            power_type_store: Arc::clone(&power_type_store),
        },
        spells: SessionSpellCatalogCapabilitiesLikeCpp {
            spell_chain_store: Arc::clone(&spell_chain_store),
            spell_store: Arc::clone(&spell_store),
            serverside_spell_store: Arc::clone(&serverside_spell_store),
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
            spell_class_options_store: Arc::clone(&spell_class_options_store),
            spell_label_store: Arc::clone(&spell_label_store),
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
            chr_specialization_store: Arc::clone(&chr_specialization_store),
            map_store: Arc::clone(&map_store),
            dungeon_encounter_store: Arc::clone(&dungeon_encounter_store),
            world_safe_loc_store: Arc::clone(&world_safe_loc_store),
            graveyards: Arc::clone(&graveyard_store),
            map_difficulty_store: Arc::clone(&map_difficulty_store),
            map_difficulty_x_condition_store: Arc::clone(&map_difficulty_x_condition_store),
            access_requirement_store: Arc::clone(&access_requirement_store),
            lfg_dungeons_store: Arc::clone(&lfg_dungeons_store),
            creature_template_lifecycle_store: Arc::clone(&creature_template_lifecycle_store),
            creature_template_mount_store: Arc::clone(&creature_template_mount_store),
            creature_display_info_store: Arc::clone(&creature_display_info_store),
            creature_display_info_extra_store: Arc::clone(&creature_display_info_extra_store),
            gameobject_display_info_store: Arc::clone(&gameobject_display_info_store),
            creature_model_info_store: Arc::clone(&creature_model_info_store),
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
        progression: session_progression_policy::build_session_progression_policy(
            &quest_xp_store,
            &quest_money_reward_store,
            &quest_store,
            &quest_v2_store,
            &quest_info_store,
            &quest_package_item_store,
            &quest_faction_reward_store,
            &progression_faction_store,
            &faction_template_store,
            &friendship_rep_reaction_store,
            &paragon_reputation_store,
            &reputation_reward_rate_store,
            &creature_onkill_reputation_store,
            &reputation_spillover_template_store,
            &world_configs,
        ),
        runtime: session_runtime_policy::build_session_runtime_policy(
            &player_registry,
            game_event_quest_complete_tx,
            &group_registry,
            &pending_invites,
            &world_configs,
        ),
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

    let RuntimeNetworkConfiguration {
        realm_addr,
        instance_addr,
        instance_port,
        max_expansion,
        mmap_runtime_config,
        mmap_pathfinder,
        creature_aggro_config,
    } = runtime_network_configuration::build_runtime_network_configuration(
        &world_configs,
        mmap_disabled_map_ids,
        &map_store,
        &data_dir,
        &locale,
        &faction_template_store,
        &progression_faction_store,
        &disable_mgr,
        &spell_misc_store,
        &spell_range_store,
        &spell_duration_store,
        &spell_cooldowns_store,
        &spell_category_store,
        &spell_x_spell_visual_store,
        &spell_target_restrictions_store,
        &spell_casting_requirements_store,
        &spell_aura_restrictions_store,
        &spell_store,
        &spell_threat_store,
        &spell_chain_store,
        &spell_linked_store,
        &condition_store,
        &spell_script_exact_spell_ids,
        &spell_script_all_rank_root_spell_ids,
        &legacy_spell_script_spell_ids,
        &spell_linked_rejected_trigger_spell_ids,
        &spell_custom_attribute_store,
        &difficulty_store,
        &creature_template_lifecycle_store,
        &chr_races_store,
    )?;

    let WorldListenerStartup {
        mut realm_handle,
        mut instance_handle,
        realm_network_abort_handle,
        instance_network_abort_handle,
    } = world_listener_startup::start_world_listeners(
        realm_addr,
        instance_addr,
        &account_lookup,
        world_listener_policy,
        &session_resources,
        &session_mgr,
        &live_terrain,
        &canonical_map_manager,
        &canonical_spawn_metadata,
        &active_session_registry,
        &world_runtime_state,
        &battle_pet_account_registry,
        instance_port,
        max_expansion,
        &mmap_runtime_config,
        &mmap_pathfinder,
        &creature_aggro_config,
    )
    .await?;

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
    // #1263 F6-8D3b-2/F6-8E: the canonical map loop is the only creature
    // runtime; the retired owner keys only warn.
    warn_retired_creature_tick_owner_keys_like_cpp();
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
    let world_module_hooks = world_module_hooks.with_realm_announcer_like_cpp(
        &player_registry,
        &active_realm.id,
        realm_id,
    );
    let mut map_update_handle = spawn_canonical_map_update_loop(
        Arc::clone(&canonical_map_manager),
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
        Arc::clone(&active_session_registry),
        Arc::clone(&battlemaster_list_typed_store),
        Arc::clone(&world_state_mgr),
        Some(CanonicalCreatureRuntimeLikeCpp::new_like_cpp(
            mmap_runtime_config.clone(),
            mmap_pathfinder.clone(),
            creature_aggro_config.clone(),
            Some(Arc::clone(&group_registry)),
            Some(Arc::clone(&live_terrain)),
        )),
        world_module_hooks.clone(),
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

    let startup_script_summary = wow_script::lifecycle::on_startup_like_cpp();
    world_module_hooks.startup_like_cpp(&worldserver_full_version_like_cpp(), u32::from(realm_id));
    info!(
        callbacks = startup_script_summary.callbacks,
        "Ran ScriptMgr::OnStartup-style lifecycle hooks"
    );

    let mut map_update_finished = false;
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
    let map_update_stopped = stop_respawn_db_producer_like_cpp(
        "canonical-map-update",
        &mut map_update_handle,
        map_update_finished,
    )
    .await;
    if !map_update_stopped {
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

    let shutdown_script_summary = wow_script::lifecycle::on_shutdown_like_cpp();
    world_module_hooks.shutdown_like_cpp(world_runtime_state.get_exit_code_like_cpp());
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
