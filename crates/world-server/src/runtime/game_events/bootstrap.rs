use super::super::*;
pub(crate) fn create_canonical_map_manager(configs: &WorldConfigSet) -> wow_map::MapManager {
    let grid_cleanup_delay_ms =
        world_config_u32(configs, "CONFIG_INTERVAL_GRIDCLEAN", 5 * 60 * 1000)
            .max(wow_map::MIN_GRID_DELAY_MS);
    let map_update_interval_ms = world_config_u32(configs, "CONFIG_INTERVAL_MAPUPDATE", 10)
        .max(wow_map::MIN_MAP_UPDATE_DELAY_MS);
    let map_update_threads = world_config_u32(configs, "CONFIG_NUMTHREADS", 1);

    let mut manager = wow_map::MapManager::new(grid_cleanup_delay_ms, map_update_interval_ms);
    if map_update_threads > 0 {
        manager
            .map_updater_mut()
            .activate(map_update_threads as usize);
    }

    info!(
        "Canonical MapManager initialized: grid_cleanup_delay_ms={}, map_update_interval_ms={}, map_update_threads={}",
        grid_cleanup_delay_ms, map_update_interval_ms, map_update_threads,
    );

    manager
}

pub(crate) fn map_db2_entries_from_stores(
    map_store: &wow_data::MapStore,
    map_difficulty_store: &wow_data::MapDifficultyStore,
    map_id: u32,
    difficulty_id: u8,
) -> Option<MapDb2Entries> {
    MapDb2Entries::from_stores_like_cpp(map_store, map_difficulty_store, map_id, difficulty_id)
}

/// Recover saved instance locks and retire obsolete chest sources before the
/// map manager registers ids or admits players (InstanceLockMgr::Load).
pub(crate) async fn load_instance_locks_and_retire_chest_sources(
    realm_id: u16,
    persistence: &dyn wow_persistence::InstanceLockPersistencePortLikeCpp,
    chest_money: &wow_database::group_loot_money_adapter::MariaDbGroupLootMoneyPersistenceAdapterLikeCpp,
    map_store: &wow_data::MapStore,
    map_difficulty_store: &wow_data::MapDifficultyStore,
) -> Result<InstanceLockMgr> {
    let (shared_instance_lock_rows, character_instance_lock_rows) =
        match persistence.load_all_like_cpp().await {
            wow_persistence::InstanceLockPersistenceLoadOutcomeLikeCpp::Loaded {
                shared_rows,
                character_rows,
            } => (shared_rows, character_rows),
            wow_persistence::InstanceLockPersistenceLoadOutcomeLikeCpp::Failed { reason } => {
                bail!("Failed to load instance locks from character database: {reason}")
            }
        };
    {
        let saved_instance_maps: std::collections::HashSet<_> = character_instance_lock_rows
            .iter()
            .map(|row| (row.map_id, row.instance_id))
            .collect();
        let retired_heads = chest_money
            .retire_unregistered_instance_chest_money_like_cpp(realm_id, |map_id, instance_id| {
                map_store
                    .get(map_id)
                    .is_some_and(|map| map.is_dungeon() || map.is_battleground_or_arena())
                    && !saved_instance_maps.contains(&(map_id, instance_id))
            })
            .await
            .context("Failed to retire chest money from unsaved instance maps")?;
        info!(
            retired_heads,
            "Retired chest money from unsaved instance maps"
        );
    }
    let mut loaded_instance_lock_mgr = InstanceLockMgr::default();
    let instance_lock_load_issues = loaded_instance_lock_mgr.load_from_rows_like_cpp(
        realm_id,
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
    Ok(loaded_instance_lock_mgr)
}

pub(crate) fn register_loaded_instance_ids(
    canonical_map_manager: &Mutex<wow_map::MapManager>,
    instance_ids: &[u32],
) {
    let Some(max_instance_id) = instance_ids.iter().copied().max() else {
        return;
    };

    match canonical_map_manager.lock() {
        Ok(mut manager) => {
            manager.init_instance_ids(u64::from(max_instance_id));
            for &instance_id in instance_ids {
                manager.register_instance_id(instance_id);
            }
        }
        Err(_) => {
            warn!("Canonical MapManager lock poisoned; persisted instance ids not registered")
        }
    }

    info!(
        "Registered {} persisted instance ids with MapManager, max_instance_id={}",
        instance_ids.len(),
        max_instance_id
    );
}
