//! Ordered runtime launch composition.

use std::sync::Arc;
use tracing::info;
use std::sync::{Mutex, atomic::AtomicBool};
use tracing::warn;
use crate::{world_config_u32, legacy_creature_global_runtime_enabled_from_config_like_cpp, DEFAULT_RESPAWN_MIN_CHECK_INTERVAL_MS, spawn_respawn_db_writer_like_cpp, spawn_canonical_map_update_loop, spawn_legacy_creature_runtime_update_loop_like_cpp, spawn_group_ready_check_tick_loop, spawn_db_keepalive_loop_like_cpp, db_keepalive_interval_minutes_like_cpp};

pub(super) struct RuntimeTasks {
    pub(super) db_keepalive_handle: Option<tokio::task::JoinHandle<()>>,
    pub(super) ready_check_tick_handle: tokio::task::JoinHandle<()>,
    pub(super) legacy_creature_runtime_handle: tokio::task::JoinHandle<()>,
    pub(super) map_update_handle: tokio::task::JoinHandle<crate::runtime::map::CanonicalMapProducerExit>,
    pub(super) respawn_db_writer_handle: tokio::task::JoinHandle<()>,
    pub(super) respawn_db_writer_tx: crate::RespawnDbWriterSenderLikeCpp,
    pub(super) respawn_db_producer_stop: Arc<std::sync::atomic::AtomicBool>,
    pub(super) respawn_db_mutation_order: Arc<std::sync::Mutex<()>>,
    pub(super) respawn_condition_interval_ms: u32,
    pub(super) legacy_creature_global_runtime_enabled: bool,
    pub(super) map_update_interval_ms: u32,
}

pub(super) fn load(
    world_configs: &wow_config::WorldConfigSet,
    instances: &super::world_instance_startup::WorldInstanceManagers,
    world_spawns: &super::world_object_startup::WorldSpawnStartup,
    condition_store: &Arc<wow_data::ConditionEntriesByTypeStore>,
    geography: &super::geography_startup::GeographyBase,
    loaded_grid_creature_respawn_caches: &crate::LoadedGridCreatureRespawnCachesLikeCpp,
    area_trigger_template_store: &Arc<wow_data::AreaTriggerTemplateStore>,
    game_event_scheduler: crate::CanonicalGameEventSchedulerLikeCpp,
    player_registry: &Arc<crate::PlayerRegistry>,
    active_session_registry: &Arc<crate::ActiveWorldSessionRegistryLikeCpp>,
    condition_references: &super::condition_reference_startup::ConditionReferences,
    world_state_mgr: &crate::SharedWorldStateMgrLikeCpp,
    mmap_runtime_config: &wow_world::MMapRuntimeConfigLikeCpp,
    mmap_pathfinder: &Option<Arc<wow_world::WorldMMapPathfinderWorkerLikeCpp>>,
    legacy_creature_aggro_config: &wow_world::session::LegacyCreatureAggroConfigLikeCpp,
    group_registry: &Arc<wow_social::group::GroupRegistry>,
    char_db: &Arc<wow_database::CharacterDatabase>,
    login_db: &Arc<wow_database::LoginDatabase>,
    world_db: &Arc<wow_database::WorldDatabase>,
) -> anyhow::Result<RuntimeTasks> {
    let map_update_interval_ms = world_config_u32(world_configs, "CONFIG_INTERVAL_MAPUPDATE", 10)
        .max(wow_map::MIN_MAP_UPDATE_DELAY_MS);
    let legacy_creature_global_runtime_enabled =
        legacy_creature_global_runtime_enabled_from_config_like_cpp();
    if legacy_creature_global_runtime_enabled {
        info!(
            map_update_interval_ms,
            "RustyCore.LegacyCreatureGlobalRuntime enabled; legacy creature tick owner set to GlobalLegacy"
        );
        match instances.shared_map.write() {
            Ok(mut manager) => {
                manager.set_tick_owner(wow_world::map_manager::RuntimeTickOwner::GlobalLegacy);
            }
            Err(_) => {
                warn!("Legacy MapManager lock poisoned; cannot enable GlobalLegacy tick owner")
            }
        }
    }
    let respawn_condition_interval_ms = world_config_u32(
        world_configs,
        "CONFIG_RESPAWN_MINCHECKINTERVALMS",
        DEFAULT_RESPAWN_MIN_CHECK_INTERVAL_MS,
    )
    .max(1);
    let respawn_db_mutation_order = Arc::new(Mutex::new(()));
    let respawn_db_producer_stop = Arc::new(AtomicBool::new(false));
    let (respawn_db_writer_tx, mut respawn_db_writer_handle) =
        spawn_respawn_db_writer_like_cpp(Arc::clone(&world_spawns.respawn_persistence));
    let mut map_update_handle = spawn_canonical_map_update_loop(
        Arc::clone(&instances.canonical_map_manager),
        Arc::clone(&instances.shared_map),
        map_update_interval_ms,
        respawn_condition_interval_ms,
        Arc::clone(&world_spawns.canonical_spawn_metadata),
        Arc::clone(condition_store),
        Arc::clone(&geography.map_store),
        Arc::clone(&world_spawns.game_event_persistence),
        respawn_db_writer_tx.clone(),
        Arc::clone(&respawn_db_mutation_order),
        Arc::clone(&respawn_db_producer_stop),
        loaded_grid_creature_respawn_caches.clone(),
        Arc::clone(area_trigger_template_store),
        game_event_scheduler,
        Arc::clone(player_registry),
        Arc::clone(active_session_registry),
        Arc::clone(&condition_references.battlemaster_list_typed_store),
        Arc::clone(world_state_mgr),
    );
    let mut legacy_creature_runtime_handle = spawn_legacy_creature_runtime_update_loop_like_cpp(
        legacy_creature_global_runtime_enabled,
        Arc::clone(&instances.shared_map),
        Arc::clone(&instances.canonical_map_manager),
        Arc::clone(&geography.map_store),
        mmap_runtime_config.clone(),
        mmap_pathfinder.clone(),
        legacy_creature_aggro_config.clone(),
        map_update_interval_ms,
        Some(respawn_db_writer_tx.clone()),
        Arc::clone(&respawn_db_mutation_order),
        Arc::clone(&respawn_db_producer_stop),
        Some(Arc::clone(group_registry)),
        Arc::clone(player_registry),
        Arc::clone(active_session_registry),
    );

    let mut ready_check_tick_handle = spawn_group_ready_check_tick_loop(
        Arc::clone(group_registry),
        Arc::clone(player_registry),
        map_update_interval_ms,
    );
    let db_keepalive_handle = spawn_db_keepalive_loop_like_cpp(
        Arc::clone(char_db),
        Arc::clone(login_db),
        Arc::clone(world_db),
        db_keepalive_interval_minutes_like_cpp(world_configs),
    );
    Ok(RuntimeTasks {
        map_update_interval_ms,
        legacy_creature_global_runtime_enabled,
        respawn_condition_interval_ms,
        respawn_db_mutation_order,
        respawn_db_producer_stop,
        respawn_db_writer_tx,
        respawn_db_writer_handle,
        map_update_handle,
        legacy_creature_runtime_handle,
        ready_check_tick_handle,
        db_keepalive_handle,
    })
}
