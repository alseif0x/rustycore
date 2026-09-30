//! Ordered world catalogs, session capabilities and the existing serving lifetime.

use std::{process::ExitCode, sync::Arc};

use anyhow::{Context, Result};
use tracing::info;
use wow_database::{
    CharacterDatabase, HotfixDatabase, ItemGuidAllocatorAdvisoryLockLikeCpp,
    LoginDatabase, WorldDatabase,
};
use wow_world::session::directory::PlayerRegistry;

use crate::{
    ActiveWorldSessionRegistryLikeCpp, WorldRuntimeStateLikeCpp,
    run_game_event_quest_complete_processor_like_cpp,
    session_resources::{SessionRealmCapabilitiesLikeCpp, SessionResources},
};
use super::{
    account_admission_startup, area_trigger_template_startup, collection_startup,
    condition_reference_startup,
    condition_startup, game_event_startup, geography_startup, group_startup,
    hotfix_delivery_startup, item_auxiliary_catalogs, jump_charge_startup, loot_startup,
    network_configuration, npc_service_catalogs, object_lookup_startup, object_query_startup,
    player_catalog_startup, player_choice_startup, player_creation_startup, presentation_startup,
    progression_catalog_startup, quest_admission_startup, realm_startup, scaling_startup,
    serve, session_catalog_capabilities, session_core_capabilities, session_persistence,
    session_runtime_policy, skill_catalogs, spell_acquisition_startup, spell_pet_startup,
    spell_world_startup, stat_tables_startup, vehicle_catalogs, world_instance_startup,
    world_object_startup, world_state_startup, world_template_startup,
};

pub(super) async fn run_world_startup(
    data_dir: &str,
    locale: &str,
    hotfix_db: &Arc<HotfixDatabase>,
    world_db: &Arc<WorldDatabase>,
    char_db: &Arc<CharacterDatabase>,
    login_db_slot: &mut Option<LoginDatabase>,
    hotfix_delivery_metadata_persistence: &dyn wow_persistence::HotfixDeliveryMetadataPersistencePortLikeCpp,
    db2_hotfix_removals: &wow_data::Db2HotfixRemovalStoreLikeCpp,
    world_ports: &super::database_startup::WorldCatalogPorts,
    player_base_stats_persistence: &dyn wow_persistence::PlayerBaseStatsPersistencePortLikeCpp,
    player_creation_catalog_persistence: &dyn wow_persistence::PlayerCreationCatalogPersistencePortLikeCpp,
    skill_world_rules_persistence: &dyn wow_persistence::SkillWorldRulesPersistencePortLikeCpp,
    static_data_overlay_persistence: &dyn wow_persistence::StaticDataOverlayPersistencePortLikeCpp,
    inventory: &super::inventory_catalogs::InventoryBaseCatalogs,
    specialization: &player_catalog_startup::SpecializationCatalog,
    dungeon_encounter_store: &Arc<wow_data::DungeonEncounterStore>,
    geography: &geography_startup::GeographyBase,
    fishing_base_skill_store: &Arc<wow_data::FishingBaseSkillStoreLikeCpp>,
    phase_store: &Arc<wow_data::PhaseStore>,
    phase_group_store: &Arc<wow_data::PhaseGroupStore>,
    phase_info_store: &mut wow_data::PhaseInfoStore,
    terrain_swap_store: &Arc<wow_data::TerrainSwapStore>,
    graveyard_slot: &mut Option<wow_data::GraveyardStore>,
    gossip_catalog_adapter: &Arc<wow_database::MariaDbGossipCatalogPersistenceAdapterLikeCpp>,
    gossip_store: &mut wow_data::GossipStore,
    world_configs: &wow_config::WorldConfigSet,
    world_runtime_state: &Arc<WorldRuntimeStateLikeCpp>,
    ip_location_store: &Arc<wow_core::IpLocationStore>,
    modules: &Arc<wow_module_api::ModuleRegistry>,
    realm_availability: &realm_startup::RealmAvailability,
    id_generators: &wow_world::session::SessionIdGeneratorsLikeCpp,
    item_guid_allocator_advisory_lock: &mut Option<ItemGuidAllocatorAdvisoryLockLikeCpp>,
) -> Result<ExitCode> {
    let mut world_templates = world_template_startup::load(
        data_dir,
        locale,
        hotfix_db,
        db2_hotfix_removals,
        world_ports,
        world_configs,
    )
    .await?;
    let emotes_store = Arc::new(
        wow_data::EmotesStore::load(data_dir, locale).context("Failed to load Emotes.db2")?,
    );
    info!("Loaded {} emote rows", emotes_store.len());
    let presentation = presentation_startup::load(data_dir, locale)?;
    let vehicles = vehicle_catalogs::load(data_dir, locale, hotfix_db, world_db).await?;
    let spawn_ids = condition_reference_startup::load_spawn_ids(
        &world_ports.world_reference_catalog_persistence,
    )
    .await?;
    let skills = skill_catalogs::load(data_dir, locale, hotfix_db, db2_hotfix_removals).await?;
    let player_catalogs = player_catalog_startup::load_player_catalogs(
        data_dir,
        locale,
        skill_world_rules_persistence,
        static_data_overlay_persistence,
    )
    .await?;
    let mut spell_pet = spell_pet_startup::load_spell_pet_startup(
        data_dir,
        locale,
        hotfix_db,
        world_db,
        db2_hotfix_removals,
        skills.skill_store.as_ref(),
        world_templates.creature_template_lifecycle_store.as_ref(),
    )
    .await?;
    let creature_addon_store = world_object_startup::load_creature_addons(
        &world_ports.world_object_catalog_persistence,
        world_templates.creature_template_lifecycle_store.as_ref(),
        spawn_ids.creature_spawn_store.as_ref(),
        world_templates.creature_runtime.creature_display_info_store.as_ref(),
        emotes_store.as_ref(),
        presentation.anim_kit_store.as_ref(),
        &spell_pet.spell_store,
        spell_pet.spell_info.spell_misc_store.as_ref(),
        spell_pet.spell_duration_store.as_ref(),
    )
    .await?;
    let (active_event_store, world_state_store) =
        world_state_startup::load_condition_world_ids(
            &world_ports.world_reference_catalog_persistence,
        )
        .await?;
    let trainer_store = condition_reference_startup::load_trainer_ids(
        &world_ports.world_reference_catalog_persistence,
    )
    .await?;
    let scaling = scaling_startup::load(data_dir, locale)?;
    let area_trigger_template_persistence =
        wow_database::MariaDbAreaTriggerTemplateCatalogPersistenceAdapterLikeCpp::new(Arc::clone(
            world_db,
        ));
    let (area_trigger_template_store, _area_trigger_template_report) =
        area_trigger_template_startup::load_area_trigger_templates(
            &area_trigger_template_persistence,
            geography.world_safe_loc_store.as_ref(),
            scaling.curve_store.as_ref(),
            &mut world_templates.script_name_interner,
        )
        .await?;
    let (map_difficulty_store, map_difficulty_x_condition_store) =
        geography_startup::load_map_difficulty_catalogs(data_dir, locale)?;
    let lfg_db2 = quest_admission_startup::load_lfg_db2(data_dir, locale, hotfix_db).await?;
    let mut world_spawns = world_object_startup::load_world_spawns(
        data_dir,
        locale,
        char_db,
        world_db,
        &world_ports.world_object_catalog_persistence,
        world_templates.creature_template_lifecycle_store.as_ref(),
        geography.map_store.as_ref(),
        map_difficulty_store.as_ref(),
        world_templates.spawn_references.spawn_group_store.as_ref(),
        area_trigger_template_store.as_ref(),
        &spell_pet.spell_store,
        world_templates.script_name_interner,
    )
    .await?;
    let (world_state_mgr, world_state_startup, world_state_mgr_report) =
        world_state_startup::load_world_state_startup(
            world_db,
            char_db,
            geography.map_store.as_ref(),
            geography.area_table_store.as_ref(),
        )
        .await?;

    let collections = collection_startup::load(data_dir, locale, hotfix_db, world_db).await?;
    let condition_references = condition_reference_startup::load(
        data_dir,
        locale,
        &world_ports.world_reference_catalog_persistence,
    )
    .await?;
    let item_search_name_store = collection_startup::load_item_search_names(data_dir, locale)?;
    let trinity_string_store = object_lookup_startup::load_localized_strings(
        &world_ports.world_auxiliary_catalog_persistence,
    )
    .await?;
    let stat_tables = stat_tables_startup::load(data_dir, locale)?;
    let transmogs = collection_startup::load_transmogs(data_dir, locale)?;
    let player_creation = player_creation_startup::load_player_creation_startup(
        data_dir,
        locale,
        player_creation_catalog_persistence,
        player_base_stats_persistence,
        &geography.map_store,
        &player_catalogs.chr_races_store,
        &player_catalogs.chr_classes_store,
        &player_catalogs.chr_model_store,
        &player_catalogs.chr_race_x_chr_model_store,
        &world_templates.gameobject_template_lifecycle_store,
        world_configs,
    )
    .await?;

    let object_queries = object_query_startup::load(
        world_db,
        world_templates.gameobject_template_lifecycle_store.as_ref(),
        world_templates.creature_template_lifecycle_store.as_ref(),
        world_spawns.item_stats_store.as_ref(),
    )
    .await?;
    let item_auxiliary = item_auxiliary_catalogs::load_item_auxiliary_catalogs(
        data_dir,
        locale,
        world_db,
        static_data_overlay_persistence,
    )
    .await?;

    let (hotfix_blob_cache, tact_key_store) = hotfix_delivery_startup::load(
        data_dir,
        locale,
        hotfix_delivery_metadata_persistence,
    )
    .await?;

    let spell_acquisition = spell_acquisition_startup::load(
        data_dir,
        locale,
        &spell_pet.spell_core_hotfix_persistence,
        &spell_pet.spell_acquisition_startup_persistence,
        db2_hotfix_removals,
        &mut spell_pet.spell_store,
        &spell_pet.spell_name_store,
        world_templates.creature_runtime.difficulty_store.as_ref(),
        skills.skill_store.as_ref(),
    )
    .await?;
    let area_triggers = area_trigger_template_startup::load_world_catalogs(
        world_db,
        geography.area_trigger_db2_store.as_ref(),
        &mut world_spawns.script_name_interner,
    )
    .await?;
    let quest_admission = quest_admission_startup::load(
        world_db,
        &world_ports.quest_catalog_persistence,
        lfg_db2.lfg_dungeons_store.as_ref(),
        map_difficulty_store.as_ref(),
    )
    .await?;
    let spell_world_catalog_persistence =
        wow_database::MariaDbSpellWorldCatalogPersistenceAdapterLikeCpp::new(Arc::clone(world_db));
    let spell_area = spell_world_startup::load_area_catalog(
        &spell_world_catalog_persistence,
        &spell_pet.spell_store,
        geography.area_table_store.as_ref(),
        quest_admission.quest_store.as_ref(),
    )
    .await?;
    let world_access = world_access_startup::load(
        &world_ports.world_auxiliary_catalog_persistence,
        &world_ports.condition_disable_catalog_persistence,
        geography,
        &map_difficulty_store,
        inventory,
        &quest_admission,
        &condition_references,
        &spell_pet.spell_store,
    )
    .await?;
    let loot = loot_startup::load(
        world_db,
        &inventory.item_store,
        world_templates.gameobject_template_lifecycle_store.as_ref(),
    )
    .await?;
    let object_lookups = object_lookup_startup::load(char_db, world_db).await?;
    let npc_services = npc_service_catalogs::load_services(
        world_db,
        &world_ports.gameplay_rule_catalog_persistence,
        &spell_pet.spell_store,
        spell_acquisition.serverside_spell_store.as_ref(),
        world_templates.creature_runtime.difficulty_store.as_ref(),
        skills.skill_line_store.as_ref(),
        world_templates.spawn_references.creature_template_store.as_ref(),
        gossip_store,
    )
    .await?;
    let battle_pet_selection = stat_tables_startup::load_battle_pet_selection(
        world_db,
        stat_tables.battle_pet_species_entry_store.as_ref(),
    )
    .await?;
    let faction_changes = progression_catalog_startup::load_faction_changes(
        &world_ports.gameplay_rule_catalog_persistence,
        &condition_references,
        &quest_admission,
        &spell_pet.spell_store,
        world_spawns.item_stats_store.as_ref(),
    )
    .await?;
    let quest_rewards = progression_catalog_startup::load_player_xp_and_quest_rewards(
        data_dir,
        locale,
        world_db,
    )
    .await?;
    let player_choice_catalog_persistence =
        wow_database::MariaDbPlayerChoiceCatalogPersistenceAdapterLikeCpp::new(Arc::clone(
            world_db,
        ));
    let (player_choice_outcome, _player_choice_locale_report) =
        player_choice_startup::load_player_choice_startup(
            &player_choice_catalog_persistence,
            condition_references.char_titles_store.as_ref(),
            quest_rewards.quest_package_item_store.as_ref(),
            skills.skill_line_store.as_ref(),
            world_spawns.item_stats_store.as_ref(),
            inventory.currency_types_store.as_ref(),
            condition_references.faction_store.as_ref(),
        )
        .await?;
    let _player_choice_store = Arc::new(player_choice_outcome.store);
    let jump_charge = jump_charge_startup::load(
        data_dir,
        locale,
        world_db,
        &spell_pet.spell_core_hotfix_persistence,
        db2_hotfix_removals,
        scaling.curve_store.as_ref(),
    )
    .await?;
    let reputation = progression_catalog_startup::load_quest_reputation_catalogs(
        data_dir,
        locale,
        world_db,
        world_templates.creature_template_lifecycle_store.as_ref(),
    )
    .await?;

    let (
        realm_identity,
        login_db,
        battle_pet_account_registry,
        table,
        account_lookup,
    ) = account_admission_startup::load(
        login_db_slot,
        realm_availability,
        &stat_tables,
    )
    .await?;

    let player_registry = Arc::new(PlayerRegistry::new());
    let active_session_registry = Arc::new(ActiveWorldSessionRegistryLikeCpp::new());
    let condition_store = condition_startup::load_world_conditions(
        &world_ports.condition_disable_catalog_persistence,
        gossip_store,
        &mut spell_pet.spell_store,
        phase_info_store,
        graveyard_slot
            .as_mut()
            .expect("Graveyard retained until condition attachment"),
        inventory,
        geography,
        &skills,
        phase_store,
        &quest_admission,
        &world_templates.spawn_references,
        &trainer_store,
        &condition_references,
        &area_trigger_template_store,
        &spawn_ids,
        &active_event_store,
        &world_state_store,
        &world_templates.creature_runtime,
        &loot,
        world_configs,
    )
    .await?;
    let graveyard_store = Arc::new(
        graveyard_slot
            .take()
            .expect("Graveyard retained until condition attachment"),
    );
    let spell_click = npc_service_catalogs::load_spell_click(
        &world_ports.gameplay_rule_catalog_persistence,
        &mut world_templates.creature_template_lifecycle_store,
        &spell_pet.spell_store,
    )
    .await?;
    let spell_world = spell_world_startup::load_world_spell_catalogs(
        data_dir,
        locale,
        &spell_world_catalog_persistence,
        &spell_pet.spell_acquisition_startup_persistence,
        db2_hotfix_removals,
        &spell_pet.spell_store,
        geography.map_store.as_ref(),
        &spell_acquisition,
        &spell_pet.spell_info,
        spell_pet.spell_procs_per_minute_store.as_ref(),
        player_catalogs.chr_races_store.as_ref(),
        world_templates.creature_runtime.creature_display_info_store.as_ref(),
        spell_area.spell_area_store.as_ref(),
        world_spawns.item_stats_store.as_ref(),
    )
    .await?;
    let spell_store = Arc::new(spell_pet.spell_store);

    let group_startup::GroupStartup {
        group_registry,
        pending_invites,
        represented_group_persistence_adapter,
        group_load_summary,
    } = group_startup::load_group_startup(
        char_db,
        world_templates.creature_runtime.difficulty_store.as_ref(),
    )
    .await?;

    let instances = world_instance_startup::load_world_instance_managers(
        data_dir,
        char_db,
        &geography.map_store,
        map_difficulty_store.as_ref(),
        world_configs,
        player_registry.as_ref(),
        &world_spawns.canonical_spawn_metadata,
        &condition_store,
        &world_spawns.persisted_respawn_times,
    )
    .await?;

    let loaded_grid_creature_respawn_caches = world_instance_startup::build_loaded_grid_caches(
        realm_availability.realm_id,
        &world_templates.creature_template_lifecycle_store,
        &world_templates.creature_template_sparring_store,
        &world_templates.creature_runtime,
        &player_catalogs,
        &world_spawns,
        &creature_addon_store,
        &jump_charge,
        &vehicles,
        &world_templates.gameobject_template_lifecycle_store,
        &world_templates.gameobject_override_lifecycle_store,
    );
    let game_event_scheduler = game_event_startup::start_system(
        &world_spawns.canonical_spawn_metadata,
        &instances.canonical_map_manager,
        &instances.shared_map,
        &loaded_grid_creature_respawn_caches,
        condition_references.battlemaster_list_typed_store.as_ref(),
        &world_state_mgr,
        player_registry.as_ref(),
        world_spawns.game_event_persistence.as_ref(),
    )
    .await?;

    let (game_event_quest_complete_tx, game_event_quest_complete_rx) = flume::bounded(1024);
    let game_event_quest_complete_handle =
        tokio::spawn(run_game_event_quest_complete_processor_like_cpp(
            game_event_quest_complete_rx,
            Arc::clone(&world_spawns.canonical_spawn_metadata),
            Arc::clone(&world_spawns.game_event_persistence),
        ));

    let world_listener_policy = network_configuration::build_listener_policy(
        world_configs,
        ip_location_store,
    );
    // The Player lifecycle port is composed here, before any session is
    // accepted, so a build that cannot persist lifecycle state fails at
    // startup rather than silently dropping offline marks at logout (#200).
    let (character_identity_cache, player_name_query_persistence_port) =
        wow_database::build_player_name_query_port_like_cpp(Arc::clone(char_db), &login_db)
            .await
            .context("Failed to load C++ character identity cache")?;
    let persistence = session_persistence::build_session_persistence_ports(
        char_db,
        &login_db,
        world_db,
        &character_identity_cache,
        &player_name_query_persistence_port,
        gossip_catalog_adapter,
        &represented_group_persistence_adapter,
        &instances.instance_lock_persistence_port,
    );
    let player_grid_loader = world_instance_startup::build_player_grid_loader(
        &instances,
        &world_spawns,
        &loaded_grid_creature_respawn_caches,
        geography,
        &area_trigger_template_store,
    );
    let session_resources = SessionResources {
        core: session_core_capabilities::build(
            object_queries.object_mgr_catalogs,
            player_grid_loader,
            geography,
            &area_triggers,
            inventory,
            &item_auxiliary,
            &player_creation,
            &player_catalogs,
            &skills,
            world_configs,
            &world_templates.creature_runtime,
            &creature_addon_store,
            &world_spawns,
            &quest_rewards,
            &battle_pet_selection.battle_pet_selection_store,
            &presentation,
            &condition_references,
            &emotes_store,
            &graveyard_store,
            &quest_admission,
            &tact_key_store,
            &hotfix_blob_cache,
            modules,
            id_generators,
            &world_templates.gameobject_template_lifecycle_store,
            &npc_services.trainer_data_store,
            &instances,
            persistence,
        ),
        inventory: session_catalog_capabilities::build_inventory(
            inventory,
            &world_spawns,
            &item_search_name_store,
            &trinity_string_store,
            &collections,
            &stat_tables,
            data_dir,
            &transmogs.transmog_set_item_store,
            &player_creation,
            &item_auxiliary,
            &loot,
        )?,
        player: session_catalog_capabilities::build_player(
            &condition_store,
            &condition_references,
            &scaling,
            &world_access.disable_mgr,
            &world_templates.creature_runtime,
            &item_auxiliary,
            &skills,
            &player_catalogs,
        ),
        spells: session_catalog_capabilities::build_spells(
            &spell_acquisition,
            &spell_store,
            &spell_world,
            &spell_pet.spell_levels_store,
            &spell_pet.spell_info,
            &spell_click.npc_spell_click_store,
            &spell_area.spell_area_store,
            &spell_pet.spell_duration_store,
            &presentation.movie_store,
            &world_spawns,
        ),
        world: session_catalog_capabilities::build_world(
            geography,
            fishing_base_skill_store,
            &specialization.chr_specialization_store,
            dungeon_encounter_store,
            &map_difficulty_store,
            &map_difficulty_x_condition_store,
            &world_access.access_requirement_store,
            &lfg_db2.lfg_dungeons_store,
            &world_templates.creature_template_lifecycle_store,
            &world_templates.creature_runtime,
            &presentation.gameobject_display_info_store,
            &collections,
            &spell_pet.spell_shapeshift_form_store,
            &vehicles,
            terrain_swap_store,
            phase_store,
            phase_group_store,
        ),
        progression: session_catalog_capabilities::build_progression(
            &quest_rewards,
            &quest_admission.quest_store,
            &reputation,
            world_configs,
        ),
        runtime: session_runtime_policy::build(
            &player_registry,
            game_event_quest_complete_tx,
            &group_registry,
            &pending_invites,
            world_configs,
        ),
        realm: SessionRealmCapabilitiesLikeCpp {
            realm_id: realm_availability.realm_id,
            realm_region: realm_identity.active_realm.id.region,
            realm_battlegroup: realm_identity.active_realm.id.site,
            realm_names: realm_identity.realm_names,
            realm_external_address: realm_identity.realm_external_address,
            realm_local_address: realm_identity.realm_local_address,
        },
    };
    let session_resources = Arc::new(session_resources);

    serve::serve(
        &session_resources,
        world_configs,
        world_runtime_state,
        data_dir,
        locale,
        world_access.mmap_disabled_map_ids,
        geography,
        &spell_pet.spell_info,
        &spell_pet.spell_cooldowns_store,
        &jump_charge,
        &condition_store,
        &account_lookup,
        world_listener_policy,
        &instances,
        &world_spawns,
        &active_session_registry,
        &battle_pet_account_registry,
        &login_db,
        char_db,
        world_db,
        realm_availability,
        &loaded_grid_creature_respawn_caches,
        &area_trigger_template_store,
        game_event_scheduler,
        &player_registry,
        &condition_references,
        &world_state_mgr,
        &group_registry,
        item_guid_allocator_advisory_lock,
        &game_event_quest_complete_handle,
    )
    .await
}
