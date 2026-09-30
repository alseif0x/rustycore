//! Encounter loot fixtures use the real shared lock manager and resident Players.
use super::*;
use wow_data::{DifficultyEntry, DifficultyStore, DungeonEncounterEntry, DungeonEncounterStore, MapDifficultyEntry, MapDifficultyStore, MapEntry, MapStore};
use wow_instances::{InstanceLockMgr, InstanceLockUpdateEvent, MapDb2Entries, ResetSchedule};

pub fn prepare_encounter_loot_players_for_test(session: &mut WorldSession, encounter: u32, players: &[ObjectGuid], locked: &[ObjectGuid]) {
    let map_id = u32::from(session.player_map_id_like_cpp());
    let maps = session.map_store().cloned().unwrap_or_else(|| Arc::new(MapStore::from_entries([MapEntry {
        id: map_id, instance_type: wow_data::map::MAP_COMMON, expansion_id: 0,
        parent_map_id: -1, cosmetic_parent_map_id: -1, flags1: 0, flags2: 0,
    }])));
    let map = *maps.get(map_id).expect("the original fixture map row");
    let difficulty = MapDifficultyEntry { id: 1, message: String::new(), map_id, difficulty_id: 0, lock_id: 0, reset_interval: 1, max_players: 40, flags: wow_data::map::MAP_DIFFICULTY_FLAG_USE_LOOT_BASED_LOCK };
    let entries = MapDb2Entries::from_resolved_entries_like_cpp(map_id, 0, &map, &difficulty);
    session.set_map_store(maps);
    session.set_map_difficulty_store(Arc::new(MapDifficultyStore::from_entries([difficulty])));
    session.set_difficulty_store(Arc::new(DifficultyStore::from_entries([DifficultyEntry { id: 0, instance_type: map.instance_type as u8, flags: 0, fallback_difficulty_id: 0, toggle_difficulty_id: 0 }])));
    session.set_dungeon_encounter_store(Arc::new(DungeonEncounterStore::from_entries([DungeonEncounterEntry { id: encounter, map_id: map_id as i16, difficulty_id: 0, order_index: 0, bit: 0, flags: 0, faction: -1 }])));
    let manager = session.canonical_map_manager.as_ref().map(Arc::clone).unwrap_or_else(|| Arc::new(std::sync::Mutex::new(wow_map::MapManager::default())));
    session.set_canonical_map_manager(Arc::clone(&manager));
    {
        let mut manager = manager.lock().expect("encounter fixture map lock");
        let map = manager.create_world_map(map_id, 0).map_mut();
        for guid in players {
            if map.get_typed_player(*guid).is_some() { continue; }
            let mut player = wow_entities::Player::new(None, false);
            player.unit_mut().world_mut().object_mut().create(*guid);
            player.unit_mut().world_mut().set_map(map_id, 0).unwrap();
            player.unit_mut().world_mut().relocate(Position::ZERO);
            player.unit_mut().world_mut().object_mut().add_to_world();
            player.unit_mut().set_max_health(1);
            player.unit_mut().set_health(1);
            map.insert_map_object_record(wow_entities::MapObjectRecord::new_player(player).unwrap()).unwrap();
        }
    }
    let locks = session.instance_lock_mgr.as_ref().map(Arc::clone).unwrap_or_else(|| Arc::new(std::sync::RwLock::new(InstanceLockMgr::default())));
    {
        let mut locks = locks.write().expect("encounter fixture lock manager");
        let now = u64::try_from(unix_now()).unwrap();
        for player in locked {
            locks.update_instance_lock_for_player_at(*player, &entries, InstanceLockUpdateEvent {
                instance_id: 0, new_data: String::new(), instance_completed_encounters_mask: 1,
                completed_encounter_bit: Some(0), entrance_world_safe_loc_id: None,
            }, ResetSchedule::default(), now).expect("real full-GUID encounter lock");
        }
    }
    session.set_instance_lock_mgr(locks);
    if session.player_guid().is_some() { let _ = session.adopt_registered_canonical_player_fixture_like_cpp(); }
}

pub fn share_encounter_loot_catalogs_for_test(source: &WorldSession, target: &mut WorldSession) {
    target.set_map_store(Arc::clone(source.map_store().unwrap()));
    target.set_map_difficulty_store(Arc::clone(source.map_difficulty_store().unwrap()));
    target.set_difficulty_store(Arc::clone(source.difficulty_store().unwrap()));
    target.set_dungeon_encounter_store(Arc::clone(source.dungeon_encounter_store().unwrap()));
    target.set_instance_lock_mgr(Arc::clone(source.instance_lock_mgr.as_ref().unwrap()));
}

