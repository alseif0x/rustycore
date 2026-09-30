//! Loaded instance locks and linked canonical/legacy world manager startup.

use std::sync::{Arc, Mutex, RwLock};

use anyhow::bail;
use tracing::{info, warn};
use wow_instances::{InstanceLockLoadIssue, InstanceLockMgr, InstanceLocksStatistics};
use wow_world::{
    MapManager as LegacyMapManager, SharedCanonicalMapManager, SharedMapManager,
    session::directory::PlayerRegistry,
};

use crate::{
    PersistedRespawnTimesLikeCpp, SharedCanonicalSpawnMetadataLikeCpp,
    respawn_bootstrap::install_canonical_spawn_group_initializer_like_cpp,
    runtime::{
        create_canonical_map_manager, map_db2_entries_from_stores, register_loaded_instance_ids,
    },
};

pub(super) struct WorldInstanceManagers {
    pub(super) canonical_map_manager: SharedCanonicalMapManager,
    pub(super) instance_lock_mgr: Arc<RwLock<InstanceLockMgr>>,
    pub(super) registered_instance_ids: Vec<u32>,
    pub(super) instance_lock_stats: InstanceLocksStatistics,
    pub(super) instance_lock_load_issues: Vec<InstanceLockLoadIssue>,
    pub(super) instance_lock_persistence_port: Arc<dyn wow_persistence::InstanceLockPersistencePortLikeCpp>,
    pub(super) shared_map: SharedMapManager,
}

pub(super) async fn load_world_instance_managers(
    data_dir: &str,
    char_db: &Arc<wow_database::CharacterDatabase>,
    map_store: &Arc<wow_data::MapStore>,
    map_difficulty_store: &wow_data::MapDifficultyStore,
    world_configs: &wow_config::WorldConfigSet,
    player_registry: &PlayerRegistry,
    canonical_spawn_metadata: &SharedCanonicalSpawnMetadataLikeCpp,
    condition_store: &Arc<wow_data::ConditionEntriesByTypeStore>,
    persisted_respawn_times: &Arc<PersistedRespawnTimesLikeCpp>,
) -> anyhow::Result<WorldInstanceManagers> {
    // Shared world state (creatures/grids visible to every session on the same map).
    // Each session gets a clone of this Arc on creation.
    let mut legacy_map_manager = LegacyMapManager::new();
    // Wire file-backed terrain so the live spawn/respawn path ground-snaps
    // creatures with real `.map` heights (issue #15). DataDir-rooted, lazy.
    legacy_map_manager.set_terrain(Arc::new(wow_world::map_manager::LiveTerrainHeights::new(
        data_dir,
    )));
    let shared_map: SharedMapManager = Arc::new(std::sync::RwLock::new(legacy_map_manager));
    let instance_lock_persistence_port: Arc<
        dyn wow_persistence::InstanceLockPersistencePortLikeCpp,
    > = Arc::new(
        wow_database::MariaDbInstanceLockPersistenceAdapterLikeCpp::new(Arc::clone(char_db)),
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
            map_db2_entries_from_stores(map_store, map_difficulty_store, map_id, difficulty_id)
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

    let canonical_map_manager = Arc::new(Mutex::new(create_canonical_map_manager(world_configs)));
    assert!(player_registry.bind_canonical_map_manager(Arc::clone(&canonical_map_manager)));
    match canonical_map_manager.lock() {
        Ok(mut manager) => install_canonical_spawn_group_initializer_like_cpp(
            &mut manager,
            Arc::clone(canonical_spawn_metadata),
            Arc::clone(condition_store),
            Arc::clone(persisted_respawn_times),
            Arc::clone(map_store),
        ),
        Err(_) => {
            warn!("Canonical MapManager lock poisoned; InitSpawnGroupState hook not installed")
        }
    }
    register_loaded_instance_ids(canonical_map_manager.as_ref(), &registered_instance_ids);

    Ok(WorldInstanceManagers {
        shared_map,
        instance_lock_persistence_port,
        instance_lock_load_issues,
        instance_lock_stats,
        registered_instance_ids,
        instance_lock_mgr,
        canonical_map_manager,
    })
}

pub(super) fn build_loaded_grid_caches(
    realm_id: u16,
    creature_template_lifecycle_store: &Arc<wow_data::CreatureTemplateLifecycleStoreLikeCpp>,
    creature_template_sparring_store: &Arc<wow_data::CreatureTemplateSparringStoreLikeCpp>,
    creature_runtime: &super::creature_catalog_startup::CreatureRuntimeCatalogs,
    player_catalogs: &super::player_catalog_startup::PlayerCatalogs,
    world_spawns: &super::world_object_startup::WorldSpawnStartup,
    creature_addon_store: &Arc<wow_data::CreatureAddonStoreLikeCpp>,
    jump_charge: &super::jump_charge_startup::JumpChargeCatalogs,
    vehicles: &super::vehicle_catalogs::VehicleCatalogs,
    gameobject_template_lifecycle_store: &Arc<wow_data::GameObjectTemplateLifecycleStoreLikeCpp>,
    gameobject_override_lifecycle_store: &Arc<wow_data::GameObjectOverrideLifecycleStoreLikeCpp>,
) -> crate::LoadedGridCreatureRespawnCachesLikeCpp {
    let loaded_grid_creature_respawn_caches = crate::LoadedGridCreatureRespawnCachesLikeCpp {
        realm_id: realm_id,
        template_store: Arc::clone(creature_template_lifecycle_store),
        sparring_store: Arc::clone(creature_template_sparring_store),
        difficulty_store: Arc::clone(&creature_runtime.creature_difficulty_store),
        base_stats_store: Arc::clone(&creature_runtime.creature_base_stats_store),
        chr_classes_store: Arc::clone(&player_catalogs.chr_classes_store),
        power_type_store: Arc::clone(&player_catalogs.power_type_store),
        health_rates: creature_runtime.creature_health_rates,
        display_store: Arc::clone(&creature_runtime.creature_display_info_store),
        model_store: Arc::clone(&creature_runtime.creature_model_data_store),
        model_info_store: Arc::clone(&creature_runtime.creature_model_info_store),
        creature_equipment_store: Arc::clone(&world_spawns.creature_equipment_store),
        creature_addon_store: Arc::clone(creature_addon_store),
        spell_x_spell_visual_store: Arc::clone(&jump_charge.spell_x_spell_visual_store),
        vehicle_store: Arc::clone(&vehicles.vehicle_store),
        vehicle_seat_store: Arc::clone(&vehicles.vehicle_seat_store),
        vehicle_accessory_store: Arc::clone(&vehicles.vehicle_accessory_store),
        gameobject_template_store: Arc::clone(gameobject_template_lifecycle_store),
        gameobject_override_store: Arc::clone(gameobject_override_lifecycle_store),
    };
    loaded_grid_creature_respawn_caches
}

pub(super) fn build_player_grid_loader(
    instances: &WorldInstanceManagers,
    world_spawns: &super::world_object_startup::WorldSpawnStartup,
    loaded_grid_creature_respawn_caches: &crate::LoadedGridCreatureRespawnCachesLikeCpp,
    geography: &super::geography_startup::GeographyBase,
    area_trigger_template_store: &Arc<wow_data::AreaTriggerTemplateStore>,
) -> wow_world::session::PlayerGridLoadResolverLikeCpp {
    let grid_canonical_map_manager = Arc::clone(&instances.canonical_map_manager);
    let grid_legacy_manager = Arc::clone(&instances.shared_map);
    let grid_spawn_metadata = Arc::clone(&world_spawns.canonical_spawn_metadata);
    let grid_loaded_caches = loaded_grid_creature_respawn_caches.clone();
    let grid_map_store = Arc::clone(&geography.map_store);
    let grid_area_trigger_template_store = Arc::clone(area_trigger_template_store);
    let player_grid_loader = Arc::new(move |map_id, instance_id, position| {
        crate::ensure_login_player_grid_loaded_like_cpp(
            &grid_canonical_map_manager,
            &grid_legacy_manager,
            &grid_spawn_metadata,
            &grid_loaded_caches,
            grid_area_trigger_template_store.as_ref(),
            Some(grid_map_store.as_ref()),
            map_id,
            instance_id,
            position,
        )
    });
    player_grid_loader
}

pub(super) fn load_dungeon_encounters(
    data_dir: &str,
    locale: &str,
) -> anyhow::Result<Arc<wow_data::DungeonEncounterStore>> {
    // Load DungeonEncounter.db2 for C++ instance encounter lock/loot metadata.
    let dungeon_encounter_store = Arc::new(
        wow_data::DungeonEncounterStore::load(data_dir, locale)
            .context("Failed to load DungeonEncounter.db2 — check DataDir and DBC.Locale config")?,
    );
    info!(
        "Loaded {} dungeon encounters from DungeonEncounter.db2",
        dungeon_encounter_store.len()
    );
    Ok(dungeon_encounter_store)
}
