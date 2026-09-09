//! Scenarios for [`super`], part 9.
//!
//! Split out of main_tests.rs under #628; assertions and registrations are
//! unchanged and shared fixtures stay in the parent module.

use super::*;

#[test]
fn respawn_db_retry_queue_coalesces_latest_and_makes_new_state_immediate() {
    let start = std::time::Instant::now();
    let mut queue = RespawnDbRetryQueueLikeCpp::default();
    queue.enqueue_latest(respawn_db_save_mutation_fixture_like_cpp(15, 100), start);
    let failed = queue.take_due(start).expect("first attempt due");
    queue.retry_failed(failed, start);

    let replacement_at = start + Duration::from_millis(10);
    queue.enqueue_latest(
        respawn_db_delete_mutation_fixture_like_cpp(15),
        replacement_at,
    );
    assert_eq!(queue.pending_len(), 1);
    let replacement = queue
        .take_due(replacement_at)
        .expect("newer same-key state must not wait behind stale backoff");
    assert!(matches!(
        replacement.pending.mutation,
        RespawnPersistenceMutationLikeCpp::Delete { .. }
    ));

    queue.enqueue_latest(
        respawn_db_save_mutation_fixture_like_cpp(15, 300),
        replacement_at,
    );
    queue.enqueue_latest(
        respawn_db_save_mutation_fixture_like_cpp(16, 400),
        replacement_at,
    );
    assert_eq!(queue.pending_len(), 2);
    assert_rep_respawn_params_like_cpp(
        &queue.pending[&respawn_persistence_key_fixture_like_cpp(15)].mutation,
        0,
        15,
        300,
        571,
        0,
    );
    assert_rep_respawn_params_like_cpp(
        &queue.pending[&respawn_persistence_key_fixture_like_cpp(16)].mutation,
        0,
        16,
        400,
        571,
        0,
    );
}
#[test]
fn respawn_db_retry_queue_shutdown_makes_existing_backoff_immediately_due() {
    let start = std::time::Instant::now();
    let mut queue = RespawnDbRetryQueueLikeCpp::default();
    queue.enqueue_latest(respawn_db_save_mutation_fixture_like_cpp(17, 100), start);
    let failed = queue.take_due(start).expect("first attempt due");
    queue.retry_failed(failed, start);

    let shutdown_at = start + Duration::from_millis(10);
    assert!(queue.take_due(shutdown_at).is_none());
    queue.make_all_due(shutdown_at);
    assert_eq!(
        queue
            .take_due(shutdown_at)
            .expect("shutdown drain must bypass stale retry deadline")
            .key
            .spawn_id,
        17
    );
}
#[tokio::test]
async fn respawn_db_writer_retries_failed_typed_mutation_then_applies_once_like_cpp() {
    let port = FakeRespawnPersistencePortLikeCpp::default();
    port.fail_mutations
        .store(true, std::sync::atomic::Ordering::Release);
    let mailbox = RespawnDbMailboxLikeCpp::default();
    let mutation = respawn_db_save_mutation_fixture_like_cpp(21, 500);
    {
        let mut state = mailbox.state.lock().unwrap();
        state.queue.enqueue_latest(mutation, Instant::now());
    }

    let first = mailbox
        .state
        .lock()
        .unwrap()
        .queue
        .take_due(Instant::now())
        .expect("fresh typed mutation is due");
    execute_respawn_db_attempt_like_cpp(first, &mailbox, &port).await;
    assert_eq!(mailbox.state.lock().unwrap().queue.pending_len(), 1);

    port.fail_mutations
        .store(false, std::sync::atomic::Ordering::Release);
    let second = {
        let mut state = mailbox.state.lock().unwrap();
        state.queue.make_all_due(Instant::now());
        state
            .queue
            .take_due(Instant::now())
            .expect("failed mutation remains retryable")
    };
    execute_respawn_db_attempt_like_cpp(second, &mailbox, &port).await;

    assert_eq!(mailbox.state.lock().unwrap().queue.pending_len(), 0);
    assert_eq!(
        port.mutations.lock().unwrap().as_slice(),
        [mutation, mutation]
    );
}
#[test]
fn spawn_group_condition_update_tick_process_respawns_delete_only_removes_inactive_due_timer() {
    let metadata = test_spawn_metadata_with_flags([(60, 571, SpawnGroupFlags::MANUAL_SPAWN)]);
    let condition_store = ConditionEntriesByTypeStore::default();
    let mut manager = wow_map::MapManager::new(60_000, 1);
    let map = manager.create_world_map(571, 0);
    map.map_mut().add_respawn_info_like_cpp(RespawnInfoLikeCpp {
        object_type: SpawnObjectType::Creature,
        spawn_id: 1,
        entry: 42,
        respawn_time: 0,
        grid_id: 7,
    });
    let mut scheduler = CanonicalRespawnConditionSchedulerLikeCpp::new(1);

    let summary = canonical_map_update_tick_set_inactive_like_cpp(
        &mut manager,
        None,
        1,
        &mut scheduler,
        &metadata,
        &condition_store,
        &canonical_test_map_store_like_cpp(),
        &empty_loaded_grid_creature_respawn_caches_like_cpp(),
    )
    .expect("scheduler fires");

    assert_eq!(summary.maps_evaluated, 1);
    assert_eq!(summary.respawn_deleted_inactive_spawn_group, 1);
    assert_eq!(summary.respawn_blocked_do_respawn_runtime, 0);
    assert_eq!(summary.respawn_db_delete_queued, 1);
    assert_eq!(summary.respawn_db_delete_skipped_non_world_map, 0);
    assert_eq!(summary.respawn_db_delete_skipped_invalid_map_id, 0);
    assert_eq!(summary.respawn_db_deletes.len(), 1);
    let delete = &summary.respawn_db_deletes[0];
    assert_eq!(delete.object_type, SpawnObjectType::Creature);
    assert_eq!(delete.spawn_id, 1);
    assert_eq!(delete.map_id, 571);
    assert_eq!(delete.instance_id, 0);
    assert_del_respawn_params_like_cpp(&delete.mutation, 0, 1, 571, 0);
    assert_eq!(
        manager
            .find_map(571, 0)
            .unwrap()
            .map()
            .get_respawn_time_like_cpp(SpawnObjectType::Creature, 1),
        0
    );
}
#[test]
fn respawn_db_save_tick_queues_linked_future_reschedule_like_cpp() {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("test clock after unix epoch")
        .as_secs() as i64;
    let linked_respawn_time = now + 3_600;
    let expected_respawn_time = linked_respawn_time + 5;
    let mut linked_respawns = LinkedRespawnStoreLikeCpp::new();
    linked_respawns.insert_like_cpp(
        linked_respawn_guid_like_cpp(wow_core::guid::HighGuid::Creature, 42, 1),
        linked_respawn_guid_like_cpp(wow_core::guid::HighGuid::Creature, 42, 2),
    );
    let metadata = test_spawn_metadata_with_flags([
        (62, 571, SpawnGroupFlags::NONE),
        (63, 571, SpawnGroupFlags::NONE),
    ])
    .with_linked_respawns_like_cpp(linked_respawns);
    let condition_store = ConditionEntriesByTypeStore::default();
    let mut manager = wow_map::MapManager::new(60_000, 1);
    let map = manager.create_world_map(571, 0);
    map.map_mut().add_respawn_info_like_cpp(RespawnInfoLikeCpp {
        object_type: SpawnObjectType::Creature,
        spawn_id: 1,
        entry: 42,
        respawn_time: 0,
        grid_id: 7,
    });
    map.map_mut().add_respawn_info_like_cpp(RespawnInfoLikeCpp {
        object_type: SpawnObjectType::Creature,
        spawn_id: 2,
        entry: 42,
        respawn_time: linked_respawn_time,
        grid_id: 8,
    });
    let mut scheduler = CanonicalRespawnConditionSchedulerLikeCpp::new(1);

    let summary = canonical_map_update_tick_set_inactive_like_cpp(
        &mut manager,
        None,
        1,
        &mut scheduler,
        &metadata,
        &condition_store,
        &canonical_test_map_store_like_cpp(),
        &empty_loaded_grid_creature_respawn_caches_like_cpp(),
    )
    .expect("scheduler fires");

    assert_eq!(summary.maps_evaluated, 1);
    assert_eq!(summary.respawn_db_save_queued, 1);
    assert_eq!(summary.respawn_db_save_skipped_non_world_map, 0);
    assert_eq!(summary.respawn_db_save_skipped_invalid_map_id, 0);
    assert_eq!(summary.respawn_db_saves.len(), 1);
    let save = &summary.respawn_db_saves[0];
    assert_eq!(save.object_type, SpawnObjectType::Creature);
    assert_eq!(save.spawn_id, 1);
    assert_eq!(save.respawn_time, expected_respawn_time);
    assert_eq!(save.map_id, 571);
    assert_eq!(save.instance_id, 0);
    assert_rep_respawn_params_like_cpp(&save.mutation, 0, 1, expected_respawn_time, 571, 0);
    let map = manager.find_map(571, 0).expect("world map");
    assert_eq!(
        map.map()
            .get_respawn_time_like_cpp(SpawnObjectType::Creature, 1),
        expected_respawn_time
    );
    assert!(
        map.map()
            .get_respawn_time_like_cpp(SpawnObjectType::Creature, 1)
            > now
    );
}
#[test]
fn canonical_gameobject_timer_replace_queues_respawn_save_before_condition_tick_like_cpp() {
    let metadata = test_spawn_metadata([]);
    let condition_store = ConditionEntriesByTypeStore::default();
    let mut manager = wow_map::MapManager::new(60_000, 1);
    manager.create_world_map(571, 0);
    let spawn_id = 77;
    let guid = test_guid_like_cpp(HighGuid::GameObject, 77, 99);
    insert_live_gameobject_for_spawn_like_cpp(&mut manager, 571, spawn_id, 77);
    {
        let gameobject = manager
            .find_map_mut(571, 0)
            .unwrap()
            .map_mut()
            .get_typed_game_object_mut(guid)
            .expect("test GameObject");
        gameobject.set_represented_gameobject_data_present_like_cpp(true);
        gameobject.set_respawn_compatibility_mode(false);
        gameobject.set_respawn_delay_time(30);
        gameobject.set_spawned_by_default(true);
        gameobject.set_loot_state(wow_entities::LootState::JustDeactivated, None);
    }
    manager
        .find_map_mut(571, 0)
        .unwrap()
        .map_mut()
        .add_respawn_info_like_cpp(RespawnInfoLikeCpp {
            object_type: SpawnObjectType::GameObject,
            spawn_id,
            entry: 99,
            respawn_time: i64::MAX,
            grid_id: 7,
        });
    // The spawn-group/ProcessRespawns timer deliberately does not fire.
    // Persisting GameObject::SaveRespawnTime belongs to Map::Update itself.
    let mut scheduler = CanonicalRespawnConditionSchedulerLikeCpp::new(100);

    let summary = canonical_map_update_tick_set_inactive_like_cpp(
        &mut manager,
        None,
        1,
        &mut scheduler,
        &metadata,
        &condition_store,
        &canonical_test_map_store_like_cpp(),
        &empty_loaded_grid_creature_respawn_caches_like_cpp(),
    )
    .expect("replaced GameObject timer must surface a DB save without waiting 100ms");

    assert_eq!(summary.maps_evaluated, 0);
    assert_eq!(summary.respawn_db_save_queued, 1);
    assert_eq!(summary.respawn_db_saves.len(), 1);
    let save = &summary.respawn_db_saves[0];
    assert_eq!(save.object_type, SpawnObjectType::GameObject);
    assert_eq!(save.spawn_id, spawn_id);
    assert!(save.respawn_time > 0);
    assert_ne!(save.respawn_time, i64::MAX);
    assert_rep_respawn_params_like_cpp(&save.mutation, 1, spawn_id, save.respawn_time, 571, 0);
    assert_eq!(
        manager
            .find_map(571, 0)
            .unwrap()
            .map()
            .get_respawn_time_like_cpp(SpawnObjectType::GameObject, spawn_id),
        save.respawn_time
    );
}
#[test]
fn canonical_gameobject_compatibility_mode_queues_db_only_respawn_save_like_cpp() {
    let metadata = test_spawn_metadata([]);
    let condition_store = ConditionEntriesByTypeStore::default();
    let mut manager = wow_map::MapManager::new(60_000, 1);
    manager.create_world_map(571, 0);
    let spawn_id = 78;
    let guid = test_guid_like_cpp(HighGuid::GameObject, 78, 99);
    insert_live_gameobject_for_spawn_like_cpp(&mut manager, 571, spawn_id, 78);
    {
        let gameobject = manager
            .find_map_mut(571, 0)
            .unwrap()
            .map_mut()
            .get_typed_game_object_mut(guid)
            .expect("test GameObject");
        gameobject.set_represented_gameobject_data_present_like_cpp(true);
        gameobject.set_respawn_compatibility_mode(true);
        gameobject.set_respawn_delay_time(30);
        gameobject.set_spawned_by_default(true);
        gameobject.set_loot_state(wow_entities::LootState::JustDeactivated, None);
    }
    let mut scheduler = CanonicalRespawnConditionSchedulerLikeCpp::new(100);

    let summary = canonical_map_update_tick_set_inactive_like_cpp(
        &mut manager,
        None,
        1,
        &mut scheduler,
        &metadata,
        &condition_store,
        &canonical_test_map_store_like_cpp(),
        &empty_loaded_grid_creature_respawn_caches_like_cpp(),
    )
    .expect("compatibility-mode GameObject must surface its DB-only save");

    assert_eq!(summary.respawn_db_save_queued, 1);
    assert_eq!(summary.respawn_db_saves.len(), 1);
    let save = &summary.respawn_db_saves[0];
    assert_eq!(save.object_type, SpawnObjectType::GameObject);
    assert_eq!(save.spawn_id, spawn_id);
    assert_rep_respawn_params_like_cpp(&save.mutation, 1, spawn_id, save.respawn_time, 571, 0);
    assert_eq!(
        manager
            .find_map(571, 0)
            .unwrap()
            .map()
            .get_respawn_time_like_cpp(SpawnObjectType::GameObject, spawn_id),
        0,
        "C++ compatibility mode writes DB directly without adding a map-owned timer"
    );
}
#[test]
fn canonical_gameobject_compatibility_save_skips_instanceable_map_like_cpp() {
    let metadata = test_spawn_metadata([]);
    let condition_store = ConditionEntriesByTypeStore::default();
    let map_id = 1_151;
    let map_store = wow_data::MapStore::from_entries([wow_data::MapEntry {
        id: map_id,
        instance_type: wow_data::map::MAP_SCENARIO,
        expansion_id: 0,
        parent_map_id: -1,
        cosmetic_parent_map_id: -1,
        flags1: wow_data::map::MAP_FLAG_GARRISON,
        flags2: 0,
    }]);
    let mut manager = wow_map::MapManager::new(60_000, 1);
    manager.create_world_map(map_id, 0);
    let spawn_id = 79;
    let guid = test_guid_like_cpp(HighGuid::GameObject, 79, 99);
    insert_live_gameobject_for_spawn_like_cpp(&mut manager, map_id, spawn_id, 79);
    {
        let gameobject = manager
            .find_map_mut(map_id, 0)
            .unwrap()
            .map_mut()
            .get_typed_game_object_mut(guid)
            .expect("test GameObject");
        gameobject.set_represented_gameobject_data_present_like_cpp(true);
        gameobject.set_respawn_compatibility_mode(true);
        gameobject.set_respawn_delay_time(30);
        gameobject.set_spawned_by_default(true);
        gameobject.set_loot_state(wow_entities::LootState::JustDeactivated, None);
    }
    let mut scheduler = CanonicalRespawnConditionSchedulerLikeCpp::new(1);

    let summary = canonical_map_update_tick_set_inactive_like_cpp(
        &mut manager,
        None,
        1,
        &mut scheduler,
        &metadata,
        &condition_store,
        &map_store,
        &empty_loaded_grid_creature_respawn_caches_like_cpp(),
    )
    .expect("scheduler fires");

    assert_eq!(summary.respawn_db_save_queued, 0);
    assert_eq!(summary.respawn_db_save_skipped_instanceable_map, 1);
    assert!(summary.respawn_db_saves.is_empty());
}
#[test]
fn spawn_group_condition_update_tick_pool_timer_uses_canonical_pool_mgr_and_queues_delete() {
    let mut pool_mgr = PoolMgrLikeCpp::new();
    pool_mgr.insert_template_like_cpp(70, PoolTemplateDataLikeCpp::new(1, 571));
    let mut group = PoolGroupLikeCpp::with_pool_id(PoolMemberKindLikeCpp::Creature, 70);
    group.add_entry_like_cpp(PoolObjectLikeCpp::new(1, 0.0), 1);
    group.add_entry_like_cpp(PoolObjectLikeCpp::new(101, 0.0), 1);
    pool_mgr
        .insert_or_replace_group_like_cpp(PoolMemberKindLikeCpp::Creature, 70, group)
        .expect("test pool group");
    pool_mgr
        .register_spawn_pool_relation_like_cpp(PoolMemberKindLikeCpp::Creature, 1, 70)
        .expect("test spawn pool relation");
    let metadata = test_spawn_metadata_with_flags([(64, 571, SpawnGroupFlags::NONE)])
        .with_pool_mgr_like_cpp(pool_mgr);
    let condition_store = ConditionEntriesByTypeStore::default();
    let mut manager = wow_map::MapManager::new(60_000, 1);
    let map = manager.create_world_map(571, 0);
    map.map_mut().add_respawn_info_like_cpp(RespawnInfoLikeCpp {
        object_type: SpawnObjectType::Creature,
        spawn_id: 1,
        entry: 42,
        respawn_time: 0,
        grid_id: 7,
    });
    let mut scheduler = CanonicalRespawnConditionSchedulerLikeCpp::new(1);

    let summary = canonical_map_update_tick_set_inactive_like_cpp(
        &mut manager,
        None,
        1,
        &mut scheduler,
        &metadata,
        &condition_store,
        &canonical_test_map_store_like_cpp(),
        &empty_loaded_grid_creature_respawn_caches_like_cpp(),
    )
    .expect("scheduler fires");

    assert_eq!(summary.maps_evaluated, 1);
    assert_eq!(summary.respawn_processed_pool_timers, 1);
    assert_eq!(summary.respawn_processed_unloaded_grid_respawns, 0);
    assert_eq!(summary.respawn_pool_update_plans, 1);
    assert_eq!(summary.respawn_blocked_pool_plan_errors, 0);
    assert_eq!(summary.respawn_blocked_pool_runtime, 0);
    assert_eq!(summary.respawn_blocked_do_respawn_runtime, 0);
    assert_eq!(summary.respawn_db_delete_queued, 1);
    assert_eq!(summary.respawn_db_deletes.len(), 1);
    let delete = &summary.respawn_db_deletes[0];
    assert_eq!(delete.object_type, SpawnObjectType::Creature);
    assert_eq!(delete.spawn_id, 1);
    assert_eq!(delete.map_id, 571);
    assert_eq!(delete.instance_id, 0);
    assert_del_respawn_params_like_cpp(&delete.mutation, 0, 1, 571, 0);
    let map = manager.find_map(571, 0).expect("world map");
    assert!(
        map.map()
            .pool_data_like_cpp()
            .is_spawned_creature_like_cpp(101)
    );
    assert_eq!(
        map.map()
            .get_respawn_time_like_cpp(SpawnObjectType::Creature, 1),
        0
    );
    assert!(
        map.map()
            .get_respawn_info_like_cpp(SpawnObjectType::Creature, 1)
            .is_none()
    );
}
#[test]
fn spawn_group_condition_update_tick_process_respawns_unloaded_grid_queues_delete_without_spawn() {
    let metadata = test_spawn_metadata_with_flags([(61, 571, SpawnGroupFlags::NONE)]);
    let condition_store = ConditionEntriesByTypeStore::default();
    let mut manager = wow_map::MapManager::new(60_000, 1);
    let map = manager.create_world_map(571, 0);
    map.map_mut().add_respawn_info_like_cpp(RespawnInfoLikeCpp {
        object_type: SpawnObjectType::Creature,
        spawn_id: 1,
        entry: 42,
        respawn_time: 0,
        grid_id: 7,
    });
    let mut scheduler = CanonicalRespawnConditionSchedulerLikeCpp::new(1);

    let summary = canonical_map_update_tick_set_inactive_like_cpp(
        &mut manager,
        None,
        1,
        &mut scheduler,
        &metadata,
        &condition_store,
        &canonical_test_map_store_like_cpp(),
        &empty_loaded_grid_creature_respawn_caches_like_cpp(),
    )
    .expect("scheduler fires");

    assert_eq!(summary.maps_evaluated, 1);
    assert_eq!(summary.respawn_deleted_inactive_spawn_group, 0);
    assert_eq!(summary.respawn_processed_unloaded_grid_respawns, 1);
    assert_eq!(summary.respawn_blocked_do_respawn_runtime, 0);
    assert_eq!(summary.respawn_db_delete_queued, 1);
    assert_eq!(summary.respawn_db_deletes.len(), 1);
    let delete = &summary.respawn_db_deletes[0];
    assert_eq!(delete.object_type, SpawnObjectType::Creature);
    assert_eq!(delete.spawn_id, 1);
    assert_eq!(delete.map_id, 571);
    assert_eq!(delete.instance_id, 0);
    assert_del_respawn_params_like_cpp(&delete.mutation, 0, 1, 571, 0);
    assert_eq!(
        manager
            .find_map(571, 0)
            .unwrap()
            .map()
            .get_respawn_time_like_cpp(SpawnObjectType::Creature, 1),
        0
    );
    assert!(
        manager
            .find_map(571, 0)
            .unwrap()
            .map()
            .get_respawn_info_like_cpp(SpawnObjectType::Creature, 1)
            .is_none()
    );
}
#[test]
fn persisted_restart_timer_respawns_once_through_canonical_owner_and_mirrors_legacy_like_cpp() {
    let spawn_id = 54_987;
    let entry = 42;
    let mut metadata =
        test_spawn_metadata_with_explicit_spawn_ids([(69, 571, SpawnGroupFlags::NONE, spawn_id)]);
    metadata = metadata.with_creature_runtime_rows_like_cpp(BTreeMap::from([(
        spawn_id,
        super::super::spawn_store_loader::CreatureSpawnRuntimeRowLikeCpp {
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
            string_id: "restart-canonical-owner".to_string(),
            spawn_time_secs: 120,
        },
    )]));
    let metadata = Arc::new(Mutex::new(metadata));
    let condition_store = Arc::new(ConditionEntriesByTypeStore::default());
    let map_store = Arc::new(canonical_test_map_store_like_cpp());
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("test clock after unix epoch")
        .as_secs() as i64;
    let mut snapshot = PersistedRespawnTimesLikeCpp::default();
    snapshot.push(
        wow_map::MapKey::new(571, 0),
        RespawnInfoLikeCpp {
            object_type: SpawnObjectType::Creature,
            spawn_id,
            entry,
            respawn_time: now.saturating_sub(1),
            grid_id: wow_map::compute_grid_coord(0.0, 0.0).get_id(),
        },
    );
    let mut manager = wow_map::MapManager::new(60_000, 1);
    install_canonical_spawn_group_initializer_like_cpp(
        &mut manager,
        Arc::clone(&metadata),
        Arc::clone(&condition_store),
        Arc::new(snapshot),
        Arc::clone(&map_store),
    );
    let map = manager.create_world_map(571, 0);
    assert!(map.map_mut().load_grid(0.0, 0.0));
    assert_eq!(map.map().map_object_count(), 0);
    assert_eq!(
        map.map()
            .get_respawn_time_like_cpp(SpawnObjectType::Creature, spawn_id),
        now.saturating_sub(1)
    );

    let legacy: wow_world::SharedMapManager =
        Arc::new(std::sync::RwLock::new(wow_world::MapManager::new()));
    let caches =
        variable_loaded_grid_creature_respawn_caches_with_vehicle_id_and_difficulty_like_cpp(
            entry, 0, 0,
        );
    let mut scheduler = CanonicalRespawnConditionSchedulerLikeCpp::new(1);
    let metadata_guard = metadata.lock().unwrap();
    let summary = canonical_map_update_tick_set_inactive_like_cpp(
        &mut manager,
        Some(&legacy),
        1,
        &mut scheduler,
        &metadata_guard,
        condition_store.as_ref(),
        map_store.as_ref(),
        &caches,
    )
    .expect("due persisted timer must run through canonical ProcessRespawns");

    assert_eq!(summary.respawn_executed_loaded_grid_respawns, 1);
    assert_eq!(summary.respawn_legacy_creature_mirrors, 1);
    assert_eq!(summary.respawn_db_delete_queued, 1);
    assert_eq!(summary.respawn_db_deletes.len(), 1);
    assert_del_respawn_params_like_cpp(
        &summary.respawn_db_deletes[0].mutation,
        0,
        spawn_id,
        571,
        0,
    );
    let creature = manager
        .find_map(571, 0)
        .unwrap()
        .map()
        .get_creature_by_spawn_id_like_cpp(spawn_id)
        .expect("canonical restart respawn");
    let creature_guid = creature.guid();
    assert!(
        legacy
            .read()
            .unwrap()
            .find_creature(571, 0, creature_guid)
            .is_some()
    );
    assert_eq!(
        manager
            .find_map(571, 0)
            .unwrap()
            .map()
            .get_respawn_time_like_cpp(SpawnObjectType::Creature, spawn_id),
        0
    );

    let second = canonical_map_update_tick_set_inactive_like_cpp(
        &mut manager,
        Some(&legacy),
        1,
        &mut scheduler,
        &metadata_guard,
        condition_store.as_ref(),
        map_store.as_ref(),
        &caches,
    )
    .expect("second scheduler tick");
    assert_eq!(second.respawn_executed_loaded_grid_respawns, 0);
    assert_eq!(second.respawn_legacy_creature_mirrors, 0);
    assert_eq!(second.respawn_db_delete_queued, 0);
    assert_eq!(
        manager
            .find_map(571, 0)
            .unwrap()
            .map()
            .creature_spawn_id_store_count_like_cpp(spawn_id),
        1
    );
}
#[test]
fn persisted_gameobject_restart_timer_respawns_once_and_queues_delete_like_cpp() {
    let spawn_id = 54_988;
    let entry = 9_001;
    let spawn = SpawnData {
        object_type: SpawnObjectType::GameObject,
        spawn_id,
        map_id: 571,
        db_data: true,
        spawn_group: SpawnGroupTemplateData::default_group(),
        id: entry,
        spawn_point: SpawnPosition::new(0.0, 0.0, 0.0, 0.0),
        phase_use_flags: 0,
        phase_id: 0,
        phase_group: 0,
        terrain_swap_map: -1,
        pool_id: 0,
        spawn_time_secs: 30,
        spawn_difficulties: vec![0],
        script_id: 0,
        string_id: String::new(),
    };
    let mut store = SpawnStore::new();
    store.add_object_spawn(&spawn, |_| false);
    let metadata = Arc::new(Mutex::new(
        super::super::spawn_store_loader::CanonicalSpawnMetadataLikeCpp::new(
            store,
            BTreeMap::new(),
        )
        .with_gameobject_runtime_rows_like_cpp(BTreeMap::from([(
            spawn_id,
            super::super::spawn_store_loader::GameObjectSpawnRuntimeRowLikeCpp {
                spawn_id,
                rotation: [0.0, 0.0, 0.0, 1.0],
                anim_progress: 55,
                state: 1,
                string_id: "restart-gameobject".to_string(),
                spawn_time_secs: 30,
            },
        )])),
    ));
    let mut data = [0; wow_entities::MAX_GAMEOBJECT_DATA];
    data[11] = 1;
    let mut caches = empty_loaded_grid_creature_respawn_caches_like_cpp();
    caches.gameobject_template_store = Arc::new(
        wow_data::GameObjectTemplateLifecycleStoreLikeCpp::from_templates([
            wow_data::GameObjectTemplateLifecycleRecordLikeCpp {
                entry,
                go_type: wow_entities::GAMEOBJECT_TYPE_GOOBER,
                display_id: 44,
                name: "Restart GameObject".to_string(),
                size: 1.0,
                data,
                content_tuning_id: 0,
                ai_name: String::new(),
                script_name: String::new(),
                string_id: String::new(),
                addon: None,
            },
        ]),
    );
    let condition_store = Arc::new(ConditionEntriesByTypeStore::default());
    let map_store = Arc::new(canonical_test_map_store_like_cpp());
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("test clock after unix epoch")
        .as_secs() as i64;
    let mut snapshot = PersistedRespawnTimesLikeCpp::default();
    snapshot.push(
        wow_map::MapKey::new(571, 0),
        RespawnInfoLikeCpp {
            object_type: SpawnObjectType::GameObject,
            spawn_id,
            entry,
            respawn_time: now.saturating_sub(1),
            grid_id: wow_map::compute_grid_coord(0.0, 0.0).get_id(),
        },
    );
    let mut manager = wow_map::MapManager::new(60_000, 1);
    install_canonical_spawn_group_initializer_like_cpp(
        &mut manager,
        Arc::clone(&metadata),
        Arc::clone(&condition_store),
        Arc::new(snapshot),
        Arc::clone(&map_store),
    );
    let map = manager.create_world_map(571, 0);
    assert!(map.map_mut().load_grid(0.0, 0.0));
    assert_eq!(map.map().map_object_count(), 0);

    let mut scheduler = CanonicalRespawnConditionSchedulerLikeCpp::new(1);
    let metadata_guard = metadata.lock().unwrap();
    let summary = canonical_map_update_tick_set_inactive_like_cpp(
        &mut manager,
        None,
        1,
        &mut scheduler,
        &metadata_guard,
        condition_store.as_ref(),
        map_store.as_ref(),
        &caches,
    )
    .expect("due persisted GameObject timer must run through ProcessRespawns");

    assert_eq!(summary.respawn_executed_loaded_grid_respawns, 1);
    assert_eq!(summary.respawn_db_delete_queued, 1);
    assert_del_respawn_params_like_cpp(
        &summary.respawn_db_deletes[0].mutation,
        1,
        spawn_id,
        571,
        0,
    );
    assert_eq!(
        manager
            .find_map(571, 0)
            .unwrap()
            .map()
            .gameobject_spawn_id_store_count_like_cpp(spawn_id),
        1
    );
    assert_eq!(
        manager
            .find_map(571, 0)
            .unwrap()
            .map()
            .get_respawn_time_like_cpp(SpawnObjectType::GameObject, spawn_id),
        0
    );

    let second = canonical_map_update_tick_set_inactive_like_cpp(
        &mut manager,
        None,
        1,
        &mut scheduler,
        &metadata_guard,
        condition_store.as_ref(),
        map_store.as_ref(),
        &caches,
    )
    .expect("second scheduler tick");
    assert_eq!(second.respawn_executed_loaded_grid_respawns, 0);
    assert_eq!(second.respawn_db_delete_queued, 0);
    assert_eq!(
        manager
            .find_map(571, 0)
            .unwrap()
            .map()
            .gameobject_spawn_id_store_count_like_cpp(spawn_id),
        1
    );
}
