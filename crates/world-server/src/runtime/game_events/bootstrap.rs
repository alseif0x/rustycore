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
    let map = map_store.get(map_id)?;
    let map_difficulty = map_difficulty_store.get(map_id, difficulty_id)?;
    Some(MapDb2Entries::from_resolved_entries_like_cpp(
        map_id,
        difficulty_id,
        map,
        map_difficulty,
    ))
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

#[cfg(test)]
mod tests {
    use super::{map_db2_entries_from_stores, register_loaded_instance_ids};
    use std::sync::Mutex;
    use wow_map::MapManager;
    use wow_data::{MapDifficultyEntry, MapDifficultyStore, MapEntry, MapStore};
    use wow_instances::{MapDb2Entries, MapDifficultyResetInterval};

    #[test]
    fn map_db2_entries_from_stores_match_cpp_fields() {
        let maps = MapStore::from_entries([MapEntry {
            id: 631,
            instance_type: wow_data::map::MAP_RAID,
            expansion_id: 0,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: wow_data::map::MAP_FLAG_FLEXIBLE_RAID_LOCKING,
            flags2: 0,
        }]);
        let difficulties = MapDifficultyStore::from_entries([MapDifficultyEntry {
            id: 900,
            message: String::new(),
            map_id: 631,
            difficulty_id: 15,
            lock_id: 7,
            reset_interval: 2,
            max_players: 25,
            flags: wow_data::map::MAP_DIFFICULTY_FLAG_USE_LOOT_BASED_LOCK,
        }]);

        let entries = map_db2_entries_from_stores(&maps, &difficulties, 631, 15).unwrap();

        assert_eq!(
            entries,
            MapDb2Entries {
                map_id: 631,
                difficulty_id: 15,
                lock_id: 7,
                reset_interval: MapDifficultyResetInterval::Weekly,
                max_players: 25,
                is_flex_locking: true,
                is_using_encounter_locks: true,
            }
        );
        assert!(map_db2_entries_from_stores(&maps, &difficulties, 631, 3).is_none());
    }

    #[test]
    fn map_db2_entries_from_stores_rejects_missing_map_and_difficulty() {
        let maps = MapStore::from_entries([]);
        let difficulties = MapDifficultyStore::from_entries([MapDifficultyEntry {
            id: 900,
            message: String::new(),
            map_id: 631,
            difficulty_id: 15,
            lock_id: 7,
            reset_interval: 2,
            max_players: 25,
            flags: 0,
        }]);

        assert!(map_db2_entries_from_stores(&maps, &difficulties, 631, 15).is_none());
    }

    #[test]
    fn persisted_instance_ids_reserve_canonical_allocator_and_empty_input_is_noop() {
        let map_manager = Mutex::new(MapManager::default());
        register_loaded_instance_ids(&map_manager, &[1, 3]);

        let mut map_manager = map_manager.lock().unwrap();
        assert_eq!(map_manager.generate_instance_id(), Some(2));
        assert_eq!(map_manager.generate_instance_id(), Some(4));
        drop(map_manager);

        let empty_map_manager = Mutex::new(MapManager::default());
        register_loaded_instance_ids(&empty_map_manager, &[]);
        assert_eq!(
            empty_map_manager
                .lock()
                .unwrap()
                .generate_instance_id(),
            Some(1)
        );
    }
}
