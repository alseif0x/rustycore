//! Scenarios for [`super`], part 2.
//!
//! Split out of main_tests.rs under #628; assertions and registrations are
//! unchanged and shared fixtures stay in the parent module.

use super::*;

#[test]
fn game_event_unspawn_positive_event_skips_guid_active_in_other_event_like_cpp() {
    let mut manager = wow_map::MapManager::default();
    manager.create_world_map(1, 0);
    let event_id = 1;
    let other_event_id = 2;
    let spawn_id = 534301;
    let mut store = SpawnStore::new();
    add_spawn_data_like_cpp(&mut store, SpawnObjectType::Creature, spawn_id, 1);
    let mut guids =
        spawn_store_loader::GameEventSpawnGuidsLikeCpp::from_game_event_max_entry_like_cpp(Some(3));
    guids = push_game_event_guid_for_test_like_cpp(
        guids,
        SpawnObjectType::Creature,
        event_id,
        spawn_id,
    );
    guids = push_game_event_guid_for_test_like_cpp(
        guids,
        SpawnObjectType::Creature,
        other_event_id,
        spawn_id,
    );
    let metadata = canonical_spawn_metadata_with_store_and_game_event_guids_like_cpp(store, guids);
    manager
        .find_map_mut(1, 0)
        .expect("test map")
        .map_mut()
        .add_respawn_info_like_cpp(respawn_info_like_cpp(
            SpawnObjectType::Creature,
            spawn_id,
            534000,
        ));
    insert_live_creature_for_spawn_like_cpp(&mut manager, 1, spawn_id, 5343011);

    let summary = game_event_unspawn_creatures_and_gameobjects_for_event_like_cpp(
        &mut manager,
        &metadata,
        &[other_event_id as u16],
        event_id,
    );

    assert_eq!(summary.creature.guids_seen, 1);
    assert_eq!(summary.creature.skipped_active_in_other_event, 1);
    assert_eq!(summary.creature.respawn_timers_removed, 0);
    assert_eq!(summary.creature.live_objects_queued, 0);
    assert!(
        manager
            .find_map(1, 0)
            .expect("test map")
            .map()
            .respawn_timer_keys_like_cpp()
            .any(|(_, timer_spawn_id)| timer_spawn_id == spawn_id)
    );
}
#[test]
fn game_event_unspawn_negative_event_does_not_apply_active_event_protection_like_cpp() {
    let mut manager = wow_map::MapManager::default();
    manager.create_world_map(1, 0);
    let event_id = -1;
    let positive_event_id = 1;
    let spawn_id = 534401;
    let mut store = SpawnStore::new();
    add_spawn_data_like_cpp(&mut store, SpawnObjectType::GameObject, spawn_id, 1);
    let mut guids =
        spawn_store_loader::GameEventSpawnGuidsLikeCpp::from_game_event_max_entry_like_cpp(Some(3));
    guids = push_game_event_guid_for_test_like_cpp(
        guids,
        SpawnObjectType::GameObject,
        event_id,
        spawn_id,
    );
    guids = push_game_event_guid_for_test_like_cpp(
        guids,
        SpawnObjectType::GameObject,
        positive_event_id,
        spawn_id,
    );
    let metadata = canonical_spawn_metadata_with_store_and_game_event_guids_like_cpp(store, guids);
    manager
        .find_map_mut(1, 0)
        .expect("test map")
        .map_mut()
        .add_respawn_info_like_cpp(respawn_info_like_cpp(
            SpawnObjectType::GameObject,
            spawn_id,
            534000,
        ));
    insert_live_gameobject_for_spawn_like_cpp(&mut manager, 1, spawn_id, 5344011);

    let summary = game_event_unspawn_creatures_and_gameobjects_for_event_like_cpp(
        &mut manager,
        &metadata,
        &[positive_event_id as u16],
        event_id,
    );

    assert_eq!(summary.gameobject.guids_seen, 1);
    assert_eq!(summary.gameobject.skipped_active_in_other_event, 0);
    assert_eq!(summary.gameobject.respawn_timers_removed, 1);
    assert_eq!(summary.gameobject.live_objects_queued, 1);
}
#[test]
fn game_event_unspawn_missing_creature_guid_list_returns_before_gameobjects_like_cpp() {
    let mut manager = wow_map::MapManager::default();
    manager.create_world_map(1, 0);
    let event_id = 99;
    let gameobject_spawn_id = 534501;
    let mut store = SpawnStore::new();
    add_spawn_data_like_cpp(
        &mut store,
        SpawnObjectType::GameObject,
        gameobject_spawn_id,
        1,
    );
    let mut guids =
        spawn_store_loader::GameEventSpawnGuidsLikeCpp::from_game_event_max_entry_like_cpp(Some(2));
    guids = push_game_event_guid_for_test_like_cpp(
        guids,
        SpawnObjectType::GameObject,
        1,
        gameobject_spawn_id,
    );
    let metadata = canonical_spawn_metadata_with_store_and_game_event_guids_like_cpp(store, guids);
    manager
        .find_map_mut(1, 0)
        .expect("test map")
        .map_mut()
        .add_respawn_info_like_cpp(respawn_info_like_cpp(
            SpawnObjectType::GameObject,
            gameobject_spawn_id,
            534000,
        ));

    let summary = game_event_unspawn_creatures_and_gameobjects_for_event_like_cpp(
        &mut manager,
        &metadata,
        &[],
        event_id,
    );

    assert_eq!(summary.event_id, event_id);
    assert!(summary.missing_event_creature_guids);
    assert!(!summary.missing_event_gameobject_guids);
    assert_eq!(summary.gameobject.guids_seen, 0);
    assert!(
        manager
            .find_map(1, 0)
            .expect("test map")
            .map()
            .respawn_timer_keys_like_cpp()
            .any(|(_, spawn_id)| spawn_id == gameobject_spawn_id)
    );
}
#[test]
fn game_event_unspawn_for_event_applies_non_pool_then_pool_like_cpp() {
    let mut manager = wow_map::MapManager::default();
    manager.create_world_map(1, 0);
    manager.create_world_map(2, 0);
    let event_id = 3;
    let creature_spawn_id = 536101;
    let gameobject_spawn_id = 536102;
    let pool_id = 536103;
    let pool_spawn_id = 536104;
    let mut store = SpawnStore::new();
    add_spawn_data_like_cpp(&mut store, SpawnObjectType::Creature, creature_spawn_id, 1);
    add_spawn_data_like_cpp(
        &mut store,
        SpawnObjectType::GameObject,
        gameobject_spawn_id,
        1,
    );
    let mut guids =
        spawn_store_loader::GameEventSpawnGuidsLikeCpp::from_game_event_max_entry_like_cpp(Some(
            10,
        ));
    guids = push_game_event_guid_for_test_like_cpp(
        guids,
        SpawnObjectType::Creature,
        event_id,
        creature_spawn_id,
    );
    guids = push_game_event_guid_for_test_like_cpp(
        guids,
        SpawnObjectType::GameObject,
        event_id,
        gameobject_spawn_id,
    );
    let game_event_pools =
        spawn_store_loader::GameEventPoolIdsLikeCpp::from_game_event_max_entry_like_cpp(Some(10))
            .with_pool_ids_for_event_like_cpp(event_id, [pool_id]);
    let metadata = spawn_store_loader::CanonicalSpawnMetadataLikeCpp::new(store, BTreeMap::new())
        .with_pool_mgr_like_cpp(pool_mgr_with_creature_pool_like_cpp(
            pool_id,
            1,
            pool_spawn_id,
        ))
        .with_game_event_pools_like_cpp(game_event_pools)
        .with_game_event_spawn_guids_like_cpp(guids);
    for (object_type, spawn_id) in [
        (SpawnObjectType::Creature, creature_spawn_id),
        (SpawnObjectType::GameObject, gameobject_spawn_id),
        (SpawnObjectType::Creature, pool_spawn_id),
    ] {
        manager
            .find_map_mut(1, 0)
            .expect("test map")
            .map_mut()
            .add_respawn_info_like_cpp(respawn_info_like_cpp(object_type, spawn_id, 536000));
    }
    insert_live_creature_for_spawn_like_cpp(&mut manager, 1, creature_spawn_id, 5361011);
    insert_live_gameobject_for_spawn_like_cpp(&mut manager, 1, gameobject_spawn_id, 5361021);
    manager
        .find_map_mut(1, 0)
        .expect("test map")
        .map_mut()
        .pool_data_mut_like_cpp()
        .add_spawn_like_cpp(SpawnObjectType::Creature, pool_spawn_id, pool_id)
        .expect("test spawned creature pool data");

    let summary = game_event_unspawn_for_event_like_cpp(&mut manager, &metadata, &[], event_id);

    assert_eq!(summary.event_id, event_id);
    assert!(!summary.pool_skipped_due_to_non_pool_bucket);
    assert!(!summary.non_pool.missing_event_creature_guids);
    assert!(!summary.non_pool.missing_event_gameobject_guids);
    assert_eq!(summary.non_pool.creature.respawn_timers_removed, 1);
    assert_eq!(summary.non_pool.creature.live_objects_queued, 1);
    assert_eq!(summary.non_pool.gameobject.respawn_timers_removed, 1);
    assert_eq!(summary.non_pool.gameobject.live_objects_queued, 1);
    assert!(!summary.pool.missing_event_pool_ids);
    assert_eq!(summary.pool.pool_summary.event_pool_ids_seen, 1);
    assert_eq!(summary.pool.pool_summary.maps_matched, 1);
    assert!(
        summary
            .pool
            .pool_summary
            .blocked_pool_plan_errors
            .is_empty()
    );
    let map = manager.find_map(1, 0).expect("test map").map();
    assert!(
        !map.pool_data_like_cpp()
            .is_spawned_creature_like_cpp(pool_spawn_id)
    );
    assert_eq!(
        map.get_respawn_time_like_cpp(SpawnObjectType::Creature, creature_spawn_id),
        0
    );
    assert_eq!(
        map.get_respawn_time_like_cpp(SpawnObjectType::GameObject, gameobject_spawn_id),
        0
    );
    let drained = manager
        .find_map_mut(1, 0)
        .expect("test map")
        .map_mut()
        .remove_all_objects_in_remove_list_like_cpp();
    assert_eq!(drained.removed, 2);
}
#[test]
fn game_event_unspawn_for_event_missing_creature_bucket_skips_gameobjects_and_pool_like_cpp() {
    let mut manager = wow_map::MapManager::default();
    manager.create_world_map(1, 0);
    let event_id = 99;
    let pool_id = 536201;
    let pool_spawn_id = 536202;
    let gameobject_spawn_id = 536203;
    let mut store = SpawnStore::new();
    add_spawn_data_like_cpp(
        &mut store,
        SpawnObjectType::GameObject,
        gameobject_spawn_id,
        1,
    );
    let guids = push_game_event_guid_for_test_like_cpp(
        spawn_store_loader::GameEventSpawnGuidsLikeCpp::from_game_event_max_entry_like_cpp(Some(2)),
        SpawnObjectType::GameObject,
        1,
        gameobject_spawn_id,
    );
    let game_event_pools =
        spawn_store_loader::GameEventPoolIdsLikeCpp::from_game_event_max_entry_like_cpp(Some(100))
            .with_pool_ids_for_event_like_cpp(event_id, [pool_id]);
    let metadata = spawn_store_loader::CanonicalSpawnMetadataLikeCpp::new(store, BTreeMap::new())
        .with_pool_mgr_like_cpp(pool_mgr_with_creature_pool_like_cpp(
            pool_id,
            1,
            pool_spawn_id,
        ))
        .with_game_event_pools_like_cpp(game_event_pools)
        .with_game_event_spawn_guids_like_cpp(guids);
    let map = manager.find_map_mut(1, 0).expect("test map").map_mut();
    map.add_respawn_info_like_cpp(respawn_info_like_cpp(
        SpawnObjectType::GameObject,
        gameobject_spawn_id,
        536200,
    ));
    map.add_respawn_info_like_cpp(respawn_info_like_cpp(
        SpawnObjectType::Creature,
        pool_spawn_id,
        536200,
    ));
    map.pool_data_mut_like_cpp()
        .add_spawn_like_cpp(SpawnObjectType::Creature, pool_spawn_id, pool_id)
        .expect("test spawned creature pool data");

    let summary = game_event_unspawn_for_event_like_cpp(&mut manager, &metadata, &[], event_id);

    assert_eq!(summary.event_id, event_id);
    assert!(summary.non_pool.missing_event_creature_guids);
    assert!(!summary.non_pool.missing_event_gameobject_guids);
    assert_eq!(summary.non_pool.gameobject.guids_seen, 0);
    assert!(summary.pool_skipped_due_to_non_pool_bucket);
    assert!(!summary.pool.missing_event_pool_ids);
    assert_eq!(summary.pool.pool_summary.event_pool_ids_seen, 0);
    let map = manager.find_map(1, 0).expect("test map").map();
    assert!(
        map.respawn_timer_keys_like_cpp()
            .any(|(_, spawn_id)| spawn_id == gameobject_spawn_id)
    );
    assert!(
        map.pool_data_like_cpp()
            .is_spawned_creature_like_cpp(pool_spawn_id)
    );
    assert!(
        map.respawn_timer_keys_like_cpp()
            .any(|(_, spawn_id)| spawn_id == pool_spawn_id)
    );
}
#[test]
fn game_event_unspawn_for_event_missing_pool_bucket_keeps_non_pool_effects_like_cpp() {
    let mut manager = wow_map::MapManager::default();
    manager.create_world_map(1, 0);
    let event_id = 99;
    let creature_spawn_id = 536301;
    let gameobject_spawn_id = 536302;
    let mut store = SpawnStore::new();
    add_spawn_data_like_cpp(&mut store, SpawnObjectType::Creature, creature_spawn_id, 1);
    add_spawn_data_like_cpp(
        &mut store,
        SpawnObjectType::GameObject,
        gameobject_spawn_id,
        1,
    );
    let mut guids =
        spawn_store_loader::GameEventSpawnGuidsLikeCpp::from_game_event_max_entry_like_cpp(Some(
            100,
        ));
    guids = push_game_event_guid_for_test_like_cpp(
        guids,
        SpawnObjectType::Creature,
        event_id,
        creature_spawn_id,
    );
    guids = push_game_event_guid_for_test_like_cpp(
        guids,
        SpawnObjectType::GameObject,
        event_id,
        gameobject_spawn_id,
    );
    let metadata = spawn_store_loader::CanonicalSpawnMetadataLikeCpp::new(store, BTreeMap::new())
        .with_game_event_pools_like_cpp(
            spawn_store_loader::GameEventPoolIdsLikeCpp::from_game_event_max_entry_like_cpp(Some(
                2,
            )),
        )
        .with_game_event_spawn_guids_like_cpp(guids);
    let map = manager.find_map_mut(1, 0).expect("test map").map_mut();
    map.add_respawn_info_like_cpp(respawn_info_like_cpp(
        SpawnObjectType::Creature,
        creature_spawn_id,
        536300,
    ));
    map.add_respawn_info_like_cpp(respawn_info_like_cpp(
        SpawnObjectType::GameObject,
        gameobject_spawn_id,
        536300,
    ));
    insert_live_creature_for_spawn_like_cpp(&mut manager, 1, creature_spawn_id, 5363011);
    insert_live_gameobject_for_spawn_like_cpp(&mut manager, 1, gameobject_spawn_id, 5363021);

    let summary = game_event_unspawn_for_event_like_cpp(&mut manager, &metadata, &[], event_id);

    assert!(!summary.pool_skipped_due_to_non_pool_bucket);
    assert_eq!(summary.non_pool.creature.respawn_timers_removed, 1);
    assert_eq!(summary.non_pool.creature.live_objects_queued, 1);
    assert_eq!(summary.non_pool.gameobject.respawn_timers_removed, 1);
    assert_eq!(summary.non_pool.gameobject.live_objects_queued, 1);
    assert!(summary.pool.missing_event_pool_ids);
    assert_eq!(summary.pool.pool_summary.event_pool_ids_seen, 0);
    let map = manager.find_map(1, 0).expect("test map").map();
    assert_eq!(
        map.get_respawn_time_like_cpp(SpawnObjectType::Creature, creature_spawn_id),
        0
    );
    assert_eq!(
        map.get_respawn_time_like_cpp(SpawnObjectType::GameObject, gameobject_spawn_id),
        0
    );
}
#[test]
fn game_event_spawn_non_pool_creature_and_gameobject_loaded_grid_adds_records_like_cpp() {
    let mut manager = wow_map::MapManager::default();
    let map = manager.create_world_map(571, 0);
    assert!(map.map_mut().load_grid(0.0, 0.0));
    let legacy_manager: wow_world::SharedMapManager =
        Arc::new(std::sync::RwLock::new(wow_world::MapManager::new()));
    let event_id = 1;
    let creature_spawn_id = 535101;
    let gameobject_spawn_id = 535201;
    let creature_entry = 42;
    let gameobject_entry = 9001;
    let mut store = SpawnStore::new();
    store.add_object_spawn(
        &game_event_spawn_test_spawn_data_like_cpp(
            SpawnObjectType::Creature,
            creature_spawn_id,
            571,
            creature_entry,
            0.0,
            0.0,
            120,
        ),
        |_| false,
    );
    store.add_object_spawn(
        &game_event_spawn_test_spawn_data_like_cpp(
            SpawnObjectType::GameObject,
            gameobject_spawn_id,
            571,
            gameobject_entry,
            0.0,
            0.0,
            30,
        ),
        |_| false,
    );
    let mut guids =
        spawn_store_loader::GameEventSpawnGuidsLikeCpp::from_game_event_max_entry_like_cpp(Some(3));
    guids = push_game_event_guid_for_test_like_cpp(
        guids,
        SpawnObjectType::Creature,
        event_id,
        creature_spawn_id,
    );
    guids = push_game_event_guid_for_test_like_cpp(
        guids,
        SpawnObjectType::GameObject,
        event_id,
        gameobject_spawn_id,
    );
    let metadata = spawn_store_loader::CanonicalSpawnMetadataLikeCpp::new(store, BTreeMap::new())
        .with_game_event_spawn_guids_like_cpp(guids)
        .with_creature_runtime_rows_like_cpp(BTreeMap::from([(
            creature_spawn_id,
            spawn_store_loader::CreatureSpawnRuntimeRowLikeCpp {
                spawn_id: creature_spawn_id,
                model_id: 999,
                equipment_id: 3,
                wander_distance: 15.0,
                curhealth: 0,
                curmana: 0,
                movement_type: 1,
                npc_flags: None,
                unit_flags: None,
                unit_flags2: None,
                unit_flags3: None,
                ground_movement_type: wow_constants::CreatureGroundMovementType::Run as u8,
                swim_allowed: true,
                flight_movement_type: 0,
                rooted: false,
                chase_movement_type: wow_constants::CreatureChaseMovementType::Run as u8,
                random_movement_type: wow_constants::CreatureRandomMovementType::Walk as u8,
                interaction_pause_timer_ms:
                    wow_entities::DEFAULT_CREATURE_INTERACTION_PAUSE_TIMER_MS_LIKE_CPP,
                string_id: "game-event-spawn-creature".to_string(),
                spawn_time_secs: 120,
            },
        )]))
        .with_gameobject_runtime_rows_like_cpp(BTreeMap::from([(
            gameobject_spawn_id,
            spawn_store_loader::GameObjectSpawnRuntimeRowLikeCpp {
                spawn_id: gameobject_spawn_id,
                rotation: [0.0, 0.0, 0.0, 1.0],
                anim_progress: 55,
                state: 1,
                string_id: "game-event-spawn-go".to_string(),
                spawn_time_secs: 30,
            },
        )]));
    let caches = game_event_spawn_test_caches_like_cpp(creature_entry, gameobject_entry);
    let map = manager.find_map_mut(571, 0).expect("test map").map_mut();
    map.add_respawn_info_like_cpp(respawn_info_like_cpp(
        SpawnObjectType::Creature,
        creature_spawn_id,
        535000,
    ));
    map.add_respawn_info_like_cpp(respawn_info_like_cpp(
        SpawnObjectType::GameObject,
        gameobject_spawn_id,
        535000,
    ));

    let summary = game_event_spawn_for_event_like_cpp(
        &mut manager,
        Some(&legacy_manager),
        &metadata,
        &caches,
        event_id,
    );

    assert_eq!(summary.event_id, event_id);
    assert!(!summary.non_pool.missing_event_creature_guids);
    assert!(!summary.non_pool.missing_event_gameobject_guids);
    assert_eq!(summary.non_pool.creature.guids_seen, 1);
    assert_eq!(summary.non_pool.creature.respawn_timers_removed, 1);
    assert_eq!(summary.non_pool.creature.load_attempts, 1);
    assert_eq!(summary.non_pool.creature.successful_loaded_grid_spawns, 1);
    assert_eq!(summary.non_pool.creature.legacy_creature_mirrors, 1);
    assert_eq!(summary.non_pool.gameobject.guids_seen, 1);
    assert_eq!(summary.non_pool.gameobject.respawn_timers_removed, 1);
    assert_eq!(summary.non_pool.gameobject.load_attempts, 1);
    assert_eq!(summary.non_pool.gameobject.successful_loaded_grid_spawns, 1);
    assert!(summary.pool.missing_event_pool_ids);
    let map = manager.find_map(571, 0).expect("test map").map();
    assert_eq!(
        map.get_respawn_time_like_cpp(SpawnObjectType::Creature, creature_spawn_id),
        0
    );
    assert_eq!(
        map.get_respawn_time_like_cpp(SpawnObjectType::GameObject, gameobject_spawn_id),
        0
    );
    let creature = map
        .get_creature_by_spawn_id_like_cpp(creature_spawn_id)
        .expect("GameEventSpawn should add loaded-grid Creature");
    assert_eq!(creature.respawn_time(), 0);
    assert!(
        legacy_manager
            .read()
            .unwrap()
            .find_creature(571, 0, creature.guid())
            .is_some(),
        "Rust split runtime must mirror C++ AddToMap-loaded creatures into the legacy tick manager"
    );
    let gameobject = map
        .get_gameobject_by_spawn_id_like_cpp(gameobject_spawn_id)
        .expect("GameEventSpawn should add spawned-by-default GameObject");
    assert_eq!(gameobject.respawn_time(), 0);
    assert!(gameobject.spawned_by_default());
}
#[test]
fn game_event_spawn_for_event_missing_creature_bucket_skips_gameobjects_and_pool_like_cpp() {
    let mut manager = wow_map::MapManager::default();
    manager.create_world_map(1, 0);
    let event_id = 99;
    let pool_id = 535901;
    let pool_spawn_id = 535902;
    let gameobject_spawn_id = 535903;
    let mut store = SpawnStore::new();
    add_spawn_data_like_cpp(&mut store, SpawnObjectType::Creature, pool_spawn_id, 1);
    add_spawn_data_like_cpp(
        &mut store,
        SpawnObjectType::GameObject,
        gameobject_spawn_id,
        1,
    );
    let mut guids =
        spawn_store_loader::GameEventSpawnGuidsLikeCpp::from_game_event_max_entry_like_cpp(Some(2));
    guids = push_game_event_guid_for_test_like_cpp(
        guids,
        SpawnObjectType::GameObject,
        1,
        gameobject_spawn_id,
    );
    let game_event_pools =
        spawn_store_loader::GameEventPoolIdsLikeCpp::from_game_event_max_entry_like_cpp(Some(100))
            .with_pool_ids_for_event_like_cpp(event_id, [pool_id]);
    let metadata = spawn_store_loader::CanonicalSpawnMetadataLikeCpp::new(store, BTreeMap::new())
        .with_pool_mgr_like_cpp(pool_mgr_with_creature_pool_like_cpp(
            pool_id,
            1,
            pool_spawn_id,
        ))
        .with_game_event_pools_like_cpp(game_event_pools)
        .with_game_event_spawn_guids_like_cpp(guids);
    let caches = empty_loaded_grid_creature_respawn_caches_like_cpp();

    let summary =
        game_event_spawn_for_event_like_cpp(&mut manager, None, &metadata, &caches, event_id);

    assert_eq!(summary.event_id, event_id);
    assert!(summary.non_pool.missing_event_creature_guids);
    assert!(!summary.non_pool.missing_event_gameobject_guids);
    assert_eq!(summary.non_pool.gameobject.guids_seen, 0);
    assert!(summary.pool_skipped_due_to_non_pool_bucket);
    assert!(!summary.pool.missing_event_pool_ids);
    assert_eq!(summary.pool.pool_summary.event_pool_ids_seen, 0);
    let map = manager.find_map(1, 0).expect("test map").map();
    assert!(
        !map.pool_data_like_cpp()
            .is_spawned_creature_like_cpp(pool_spawn_id)
    );
    assert_eq!(
        map.get_respawn_time_like_cpp(SpawnObjectType::GameObject, gameobject_spawn_id),
        0
    );
}
#[test]
fn game_event_spawn_for_event_missing_gameobject_bucket_skips_pool_after_creatures_like_cpp() {
    let mut manager = wow_map::MapManager::default();
    let map = manager.create_world_map(571, 0);
    assert!(map.map_mut().load_grid(0.0, 0.0));
    manager.create_world_map(1, 0);
    let event_id = 7;
    let creature_spawn_id = 535904;
    let pool_id = 535905;
    let pool_spawn_id = 535906;
    let creature_entry = 42;
    let mut store = SpawnStore::new();
    store.add_object_spawn(
        &game_event_spawn_test_spawn_data_like_cpp(
            SpawnObjectType::Creature,
            creature_spawn_id,
            571,
            creature_entry,
            0.0,
            0.0,
            120,
        ),
        |_| false,
    );
    add_spawn_data_like_cpp(&mut store, SpawnObjectType::Creature, pool_spawn_id, 1);
    let mut guids =
        spawn_store_loader::GameEventSpawnGuidsLikeCpp::from_game_event_max_entry_like_cpp(Some(
            10,
        ));
    guids = push_game_event_guid_for_test_like_cpp(
        guids,
        SpawnObjectType::Creature,
        event_id,
        creature_spawn_id,
    )
    .truncate_gameobject_guid_buckets_for_test_like_cpp(17);
    let game_event_pools =
        spawn_store_loader::GameEventPoolIdsLikeCpp::from_game_event_max_entry_like_cpp(Some(10))
            .with_pool_ids_for_event_like_cpp(event_id, [pool_id]);
    let metadata = spawn_store_loader::CanonicalSpawnMetadataLikeCpp::new(store, BTreeMap::new())
        .with_pool_mgr_like_cpp(pool_mgr_with_creature_pool_like_cpp(
            pool_id,
            1,
            pool_spawn_id,
        ))
        .with_game_event_pools_like_cpp(game_event_pools)
        .with_game_event_spawn_guids_like_cpp(guids)
        .with_creature_runtime_rows_like_cpp(BTreeMap::from([(
            creature_spawn_id,
            spawn_store_loader::CreatureSpawnRuntimeRowLikeCpp {
                spawn_id: creature_spawn_id,
                model_id: 999,
                equipment_id: 3,
                wander_distance: 15.0,
                curhealth: 0,
                curmana: 0,
                movement_type: 1,
                npc_flags: None,
                unit_flags: None,
                unit_flags2: None,
                unit_flags3: None,
                ground_movement_type: wow_constants::CreatureGroundMovementType::Run as u8,
                swim_allowed: true,
                flight_movement_type: 0,
                rooted: false,
                chase_movement_type: wow_constants::CreatureChaseMovementType::Run as u8,
                random_movement_type: wow_constants::CreatureRandomMovementType::Walk as u8,
                interaction_pause_timer_ms:
                    wow_entities::DEFAULT_CREATURE_INTERACTION_PAUSE_TIMER_MS_LIKE_CPP,
                string_id: "game-event-spawn-creature-before-missing-go".to_string(),
                spawn_time_secs: 120,
            },
        )]));
    let caches = game_event_spawn_test_caches_like_cpp(creature_entry, 9001);
    manager
        .find_map_mut(571, 0)
        .expect("test map")
        .map_mut()
        .add_respawn_info_like_cpp(respawn_info_like_cpp(
            SpawnObjectType::Creature,
            creature_spawn_id,
            535000,
        ));

    let summary =
        game_event_spawn_for_event_like_cpp(&mut manager, None, &metadata, &caches, event_id);

    assert_eq!(summary.event_id, event_id);
    assert!(!summary.non_pool.missing_event_creature_guids);
    assert!(summary.non_pool.missing_event_gameobject_guids);
    assert_eq!(summary.non_pool.creature.guids_seen, 1);
    assert_eq!(summary.non_pool.creature.respawn_timers_removed, 1);
    assert_eq!(summary.non_pool.creature.successful_loaded_grid_spawns, 1);
    assert_eq!(summary.non_pool.gameobject.guids_seen, 0);
    assert!(summary.pool_skipped_due_to_non_pool_bucket);
    assert!(!summary.pool.missing_event_pool_ids);
    assert_eq!(summary.pool.pool_summary.event_pool_ids_seen, 0);
    let creature_map = manager.find_map(571, 0).expect("creature map").map();
    assert!(
        creature_map
            .get_creature_by_spawn_id_like_cpp(creature_spawn_id)
            .is_some()
    );
    let pool_map = manager.find_map(1, 0).expect("pool map").map();
    assert!(
        !pool_map
            .pool_data_like_cpp()
            .is_spawned_creature_like_cpp(pool_spawn_id)
    );
}
#[test]
fn game_event_spawn_non_pool_unloaded_grid_removes_timer_without_fabricating_like_cpp() {
    let mut manager = wow_map::MapManager::default();
    manager.create_world_map(571, 0);
    let event_id = 1;
    let spawn_id = 535301;
    let entry = 42;
    let mut store = SpawnStore::new();
    store.add_object_spawn(
        &game_event_spawn_test_spawn_data_like_cpp(
            SpawnObjectType::Creature,
            spawn_id,
            571,
            entry,
            1_000.0,
            1_000.0,
            120,
        ),
        |_| false,
    );
    let guids = push_game_event_guid_for_test_like_cpp(
        spawn_store_loader::GameEventSpawnGuidsLikeCpp::from_game_event_max_entry_like_cpp(Some(2)),
        SpawnObjectType::Creature,
        event_id,
        spawn_id,
    );
    let metadata = spawn_store_loader::CanonicalSpawnMetadataLikeCpp::new(store, BTreeMap::new())
        .with_game_event_spawn_guids_like_cpp(guids)
        .with_creature_runtime_rows_like_cpp(BTreeMap::from([(
            spawn_id,
            spawn_store_loader::CreatureSpawnRuntimeRowLikeCpp {
                spawn_id,
                model_id: 999,
                equipment_id: 3,
                wander_distance: 15.0,
                curhealth: 0,
                curmana: 0,
                movement_type: 1,
                npc_flags: None,
                unit_flags: None,
                unit_flags2: None,
                unit_flags3: None,
                ground_movement_type: wow_constants::CreatureGroundMovementType::Run as u8,
                swim_allowed: true,
                flight_movement_type: 0,
                rooted: false,
                chase_movement_type: wow_constants::CreatureChaseMovementType::Run as u8,
                random_movement_type: wow_constants::CreatureRandomMovementType::Walk as u8,
                interaction_pause_timer_ms:
                    wow_entities::DEFAULT_CREATURE_INTERACTION_PAUSE_TIMER_MS_LIKE_CPP,
                string_id: "game-event-unloaded-creature".to_string(),
                spawn_time_secs: 120,
            },
        )]));
    let caches = game_event_spawn_test_caches_like_cpp(entry, 9001);
    manager
        .find_map_mut(571, 0)
        .expect("test map")
        .map_mut()
        .add_respawn_info_like_cpp(respawn_info_like_cpp(
            SpawnObjectType::Creature,
            spawn_id,
            535000,
        ));

    let summary = game_event_spawn_creatures_and_gameobjects_for_event_like_cpp(
        &mut manager,
        None,
        &metadata,
        &caches,
        event_id,
    );

    assert_eq!(summary.creature.guids_seen, 1);
    assert_eq!(summary.creature.maps_matched, 1);
    assert_eq!(summary.creature.respawn_timers_removed, 1);
    assert_eq!(summary.creature.unloaded_grid_skips, 1);
    assert_eq!(summary.creature.load_attempts, 0);
    assert_eq!(summary.creature.successful_loaded_grid_spawns, 0);
    let map = manager.find_map(571, 0).expect("test map").map();
    assert_eq!(
        map.get_respawn_time_like_cpp(SpawnObjectType::Creature, spawn_id),
        0
    );
    assert!(map.get_creature_by_spawn_id_like_cpp(spawn_id).is_none());
}
#[test]
fn game_event_spawn_missing_creature_bucket_returns_before_gameobjects_like_cpp() {
    let mut manager = wow_map::MapManager::default();
    let map = manager.create_world_map(571, 0);
    assert!(map.map_mut().load_grid(0.0, 0.0));
    let event_id = 99;
    let gameobject_spawn_id = 535401;
    let gameobject_entry = 9001;
    let mut store = SpawnStore::new();
    store.add_object_spawn(
        &game_event_spawn_test_spawn_data_like_cpp(
            SpawnObjectType::GameObject,
            gameobject_spawn_id,
            571,
            gameobject_entry,
            0.0,
            0.0,
            30,
        ),
        |_| false,
    );
    let guids = push_game_event_guid_for_test_like_cpp(
        spawn_store_loader::GameEventSpawnGuidsLikeCpp::from_game_event_max_entry_like_cpp(Some(2)),
        SpawnObjectType::GameObject,
        1,
        gameobject_spawn_id,
    );
    let metadata = spawn_store_loader::CanonicalSpawnMetadataLikeCpp::new(store, BTreeMap::new())
        .with_game_event_spawn_guids_like_cpp(guids)
        .with_gameobject_runtime_rows_like_cpp(BTreeMap::from([(
            gameobject_spawn_id,
            spawn_store_loader::GameObjectSpawnRuntimeRowLikeCpp {
                spawn_id: gameobject_spawn_id,
                rotation: [0.0, 0.0, 0.0, 1.0],
                anim_progress: 55,
                state: 1,
                string_id: "game-event-missing-creature-bucket-go".to_string(),
                spawn_time_secs: 30,
            },
        )]));
    let caches = game_event_spawn_test_caches_like_cpp(42, gameobject_entry);
    manager
        .find_map_mut(571, 0)
        .expect("test map")
        .map_mut()
        .add_respawn_info_like_cpp(respawn_info_like_cpp(
            SpawnObjectType::GameObject,
            gameobject_spawn_id,
            535000,
        ));

    let summary = game_event_spawn_creatures_and_gameobjects_for_event_like_cpp(
        &mut manager,
        None,
        &metadata,
        &caches,
        event_id,
    );

    assert_eq!(summary.event_id, event_id);
    assert!(summary.missing_event_creature_guids);
    assert_eq!(summary.gameobject.guids_seen, 0);
    let map = manager.find_map(571, 0).expect("test map").map();
    assert_eq!(
        map.get_respawn_time_like_cpp(SpawnObjectType::GameObject, gameobject_spawn_id),
        535000
    );
    assert!(
        map.get_gameobject_by_spawn_id_like_cpp(gameobject_spawn_id)
            .is_none()
    );
}
