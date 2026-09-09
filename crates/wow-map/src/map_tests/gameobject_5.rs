//! Gameobject scenarios for [`super`].
//!
//! Split out of map_tests.rs under #628; assertions and
//! registrations are unchanged and shared fixtures stay in the parent module.

use super::*;

#[test]
fn gameobject_update_owner_or_spell_with_future_respawn_does_not_delete_like_cpp() {
    let mut map = test_map();
    let mut gameobject = game_object_with_counter(4620601, 571, 7, false);
    let gameobject_guid = gameobject.world().guid();
    gameobject.set_go_type(GAMEOBJECT_TYPE_CHEST as u8);
    gameobject.set_represented_chest_loot_source_like_cpp(Some(GameObjectLootSource {
        chest_consumable: true,
        ..GameObjectLootSource::default()
    }));
    gameobject.set_created_by(guid(HighGuid::Player, 4620691));
    gameobject.set_spell_id(456);
    gameobject.set_respawn_time(60);
    gameobject.set_respawn_delay_time(0);
    gameobject.set_loot_state(LootState::JustDeactivated, None);

    map.add_map_object_record_to_map_like_cpp(
        MapObjectRecord::new_game_object(gameobject).unwrap(),
    )
    .unwrap();

    let outcome = map.update_game_object_like_cpp(gameobject_guid, 1, 1_000);
    let canonical = map
        .map_object_record(gameobject_guid)
        .and_then(MapObjectRecord::game_object)
        .unwrap();

    assert_eq!(outcome.status, GameObjectUpdateStatusLikeCpp::Updated);
    assert!(outcome.loot_cleared);
    assert!(!outcome.summoned_expired_delete);
    assert!(!outcome.non_consumed_chest_or_goober_return);
    assert!(outcome.generic_not_ready);
    assert!(outcome.generic_visual_despawn_represented);
    assert_eq!(canonical.respawn_time(), 60);
    assert_eq!(canonical.loot_state(), LootState::NotReady);
    assert_eq!(map.objects_to_remove_count_like_cpp(), 0);
}
#[test]
fn gameobject_update_generic_spawned_default_noncompat_schedules_respawn_and_remove_like_cpp() {
    let mut map = test_map();
    let mut gameobject = game_object_with_counter(4640101, 571, 7, false);
    let gameobject_guid = gameobject.world().guid();
    gameobject.set_go_type(GAMEOBJECT_TYPE_GENERIC_LIKE_CPP as u8);
    gameobject.world_mut().object_mut().set_entry(190001);
    gameobject.set_spawn_id(4640101);
    gameobject.set_spawned_by_default(true);
    gameobject.set_represented_gameobject_data_present_like_cpp(true);
    gameobject.set_respawn_compatibility_mode(false);
    gameobject.set_respawn_delay_time(45);
    gameobject.set_loot_state(LootState::JustDeactivated, None);

    map.add_map_object_record_to_map_like_cpp(
        MapObjectRecord::new_game_object(gameobject).unwrap(),
    )
    .unwrap();

    let outcome = map.update_game_object_like_cpp(gameobject_guid, 1, 1_000);
    let canonical = map
        .map_object_record(gameobject_guid)
        .and_then(MapObjectRecord::game_object)
        .unwrap();
    let respawn_info = map
        .get_respawn_info_like_cpp(SpawnObjectType::GameObject, 4640101)
        .unwrap();

    assert_eq!(
        outcome.status,
        GameObjectUpdateStatusLikeCpp::DespawnRemoveQueued
    );
    assert!(outcome.generic_not_ready);
    assert_eq!(outcome.generic_respawn_scheduled_time, Some(1_045));
    assert!(outcome.generic_spawned_by_default_branch);
    assert_eq!(
        outcome.generic_respawn_timer_add,
        Some(AddRespawnInfoOutcomeLikeCpp::Inserted)
    );
    assert!(!outcome.generic_respawn_compatibility_db_only_represented);
    assert!(!outcome.generic_visibility_on_destroy_represented);
    assert!(outcome.remove_list.is_some());
    assert_eq!(canonical.respawn_time(), 1_045);
    assert_eq!(canonical.loot_state(), LootState::NotReady);
    assert_eq!(respawn_info.object_type, SpawnObjectType::GameObject);
    assert_eq!(respawn_info.spawn_id, 4640101);
    assert_eq!(respawn_info.entry, 190001);
    assert_eq!(respawn_info.respawn_time, 1_045);
    assert_eq!(respawn_info.grid_id, compute_grid_coord(1.0, 2.0).get_id());
    assert_eq!(map.objects_to_remove_count_like_cpp(), 1);
}
#[test]
fn gameobject_update_with_pool_metadata_uses_delete_pool_branch_like_cpp() {
    let mut map = test_map();
    let mut store = SpawnStore::new();
    let active = spawn_group(4640110, SpawnGroupFlags::NONE);
    let mut trigger_spawn = spawn_data(SpawnObjectType::GameObject, 4640111, active.clone());
    trigger_spawn.pool_id = 464;
    let mut replacement_spawn = spawn_data(SpawnObjectType::GameObject, 4640112, active);
    replacement_spawn.pool_id = 464;
    store.add_object_spawn(&trigger_spawn, |_| false);
    store.add_object_spawn(&replacement_spawn, |_| false);

    let mut pool_mgr = PoolMgrLikeCpp::new();
    pool_mgr.insert_template_like_cpp(464, PoolTemplateDataLikeCpp::new(1, 571));
    let mut group = PoolGroupLikeCpp::with_pool_id(PoolMemberKindLikeCpp::GameObject, 464);
    group.add_entry_like_cpp(PoolObjectLikeCpp::new(4640112, 0.0), 1);
    group.add_entry_like_cpp(PoolObjectLikeCpp::new(4640111, 0.0), 1);
    pool_mgr
        .insert_or_replace_group_like_cpp(PoolMemberKindLikeCpp::GameObject, 464, group)
        .expect("test pool group");
    pool_mgr
        .register_spawn_pool_relation_like_cpp(PoolMemberKindLikeCpp::GameObject, 4640111, 464)
        .expect("test pool relation");

    let mut gameobject = game_object_with_counter(4640111, 571, 7, false);
    let gameobject_guid = gameobject.world().guid();
    gameobject.set_go_type(GAMEOBJECT_TYPE_GENERIC_LIKE_CPP as u8);
    gameobject.world_mut().object_mut().set_entry(190_011);
    gameobject.set_spawn_id(4640111);
    gameobject.set_represented_gameobject_data_present_like_cpp(true);
    gameobject.set_respawn_compatibility_mode(true);
    gameobject.set_created_by(guid(HighGuid::Player, 4640199));
    gameobject.set_respawn_time(0);
    gameobject.set_loot_state(LootState::JustDeactivated, None);
    map.add_map_object_record_to_map_like_cpp(
        MapObjectRecord::new_game_object(gameobject).unwrap(),
    )
    .unwrap();
    map.pool_data_mut_like_cpp()
        .add_spawn_like_cpp(SpawnObjectType::GameObject, 4640111, 464)
        .expect("trigger spawned in pool");

    let outcome = map.update_game_object_with_pool_update_like_cpp(
        gameobject_guid,
        1,
        1_000,
        &store,
        &pool_mgr,
    );

    assert_eq!(
        outcome.status,
        GameObjectUpdateStatusLikeCpp::DespawnPoolUpdated
    );
    assert!(outcome.summoned_expired_delete);
    assert!(outcome.summoned_expired_respawn_time_zeroed);
    assert!(outcome.summoned_expired_despawn_represented);
    assert!(outcome.remove_list.is_none());
    assert_eq!(map.objects_to_remove_count_like_cpp(), 0);
    assert!(map.map_object_record(gameobject_guid).is_none());
    assert!(
        map.pool_data_like_cpp()
            .is_spawned_gameobject_like_cpp(4640112)
    );
    assert_eq!(
        map.pool_data_like_cpp().get_spawned_objects_like_cpp(464),
        1
    );
}
#[test]
fn gameobject_delete_pool_update_loaded_grid_loader_adds_replacement_like_cpp() {
    let mut map = test_map();
    let mut store = SpawnStore::new();
    let active = spawn_group(4640120, SpawnGroupFlags::NONE);
    let mut trigger_spawn = spawn_data(SpawnObjectType::GameObject, 4640121, active.clone());
    trigger_spawn.pool_id = 464;
    let mut replacement_spawn = spawn_data(SpawnObjectType::GameObject, 4640122, active);
    replacement_spawn.pool_id = 464;
    store.add_object_spawn(&trigger_spawn, |_| false);
    store.add_object_spawn(&replacement_spawn, |_| false);

    let mut pool_mgr = PoolMgrLikeCpp::new();
    pool_mgr.insert_template_like_cpp(464, PoolTemplateDataLikeCpp::new(1, 571));
    let mut group = PoolGroupLikeCpp::with_pool_id(PoolMemberKindLikeCpp::GameObject, 464);
    group.add_entry_like_cpp(PoolObjectLikeCpp::new(4640122, 0.0), 1);
    group.add_entry_like_cpp(PoolObjectLikeCpp::new(4640121, 0.0), 1);
    pool_mgr
        .insert_or_replace_group_like_cpp(PoolMemberKindLikeCpp::GameObject, 464, group)
        .expect("test pool group");
    pool_mgr
        .register_spawn_pool_relation_like_cpp(PoolMemberKindLikeCpp::GameObject, 4640121, 464)
        .expect("test pool relation");

    map.ensure_grid_loaded(&cell_from_world(1.0, 2.0));
    let mut gameobject = game_object_with_counter(4640121, 571, 7, false);
    let gameobject_guid = gameobject.world().guid();
    gameobject.set_go_type(GAMEOBJECT_TYPE_GENERIC_LIKE_CPP as u8);
    gameobject.world_mut().object_mut().set_entry(190_121);
    gameobject.set_spawn_id(4640121);
    gameobject.set_represented_gameobject_data_present_like_cpp(true);
    gameobject.set_respawn_compatibility_mode(true);
    gameobject.set_respawn_time(0);
    gameobject.set_loot_state(LootState::JustDeactivated, None);
    map.add_map_object_record_to_map_like_cpp(
        MapObjectRecord::new_game_object(gameobject).unwrap(),
    )
    .unwrap();
    map.pool_data_mut_like_cpp()
        .add_spawn_like_cpp(SpawnObjectType::GameObject, 4640121, 464)
        .expect("trigger spawned in pool");

    let replacement_guid = guid(HighGuid::GameObject, 4640122);
    let mut loader_calls = 0usize;
    let outcome = map
        .gameobject_delete_with_pool_update_loaded_grid_records_like_cpp(
            gameobject_guid,
            &store,
            &pool_mgr,
            |_, _| 0.0,
            |_candidates, count| (0..count).collect(),
            |_, object_type, spawn_id| {
                loader_calls += 1;
                assert_eq!(object_type, SpawnObjectType::GameObject);
                assert_eq!(spawn_id, 4640122);
                Some(LoadedGridRespawnRecordsLikeCpp::primary_only(
                    MapObjectRecord::new_game_object(test_gameobject_for_spawn(spawn_id, 4640122))
                        .unwrap(),
                ))
            },
        )
        .expect("pool-aware delete outcome");

    assert_eq!(loader_calls, 1);
    assert!(outcome.pool_update_represented);
    let summary = outcome.pool_update_summary.expect("pool update summary");
    assert_eq!(summary.executed_loaded_grid_respawns, 1);
    assert_eq!(summary.pool_spawn_actions_blocked_loaded_grid, 0);
    assert_eq!(summary.pool_spawn_actions_skipped_unloaded_grid, 0);
    assert_eq!(summary.pool_spawn_action_load_plans, vec![]);
    assert!(map.map_object_record(replacement_guid).is_some());
    assert_eq!(map.gameobject_spawn_id_store_count_like_cpp(4640122), 1);
    assert!(
        map.pool_data_like_cpp()
            .is_spawned_gameobject_like_cpp(4640122)
    );
}
#[test]
fn gameobject_update_pool_update_loaded_grid_loader_adds_replacement_like_cpp() {
    let mut map = test_map();
    let mut store = SpawnStore::new();
    let active = spawn_group(4640130, SpawnGroupFlags::NONE);
    let mut trigger_spawn = spawn_data(SpawnObjectType::GameObject, 4640131, active.clone());
    trigger_spawn.pool_id = 464;
    let mut replacement_spawn = spawn_data(SpawnObjectType::GameObject, 4640132, active);
    replacement_spawn.pool_id = 464;
    store.add_object_spawn(&trigger_spawn, |_| false);
    store.add_object_spawn(&replacement_spawn, |_| false);

    let mut pool_mgr = PoolMgrLikeCpp::new();
    pool_mgr.insert_template_like_cpp(464, PoolTemplateDataLikeCpp::new(1, 571));
    let mut group = PoolGroupLikeCpp::with_pool_id(PoolMemberKindLikeCpp::GameObject, 464);
    group.add_entry_like_cpp(PoolObjectLikeCpp::new(4640132, 0.0), 1);
    group.add_entry_like_cpp(PoolObjectLikeCpp::new(4640131, 0.0), 1);
    pool_mgr
        .insert_or_replace_group_like_cpp(PoolMemberKindLikeCpp::GameObject, 464, group)
        .expect("test pool group");
    pool_mgr
        .register_spawn_pool_relation_like_cpp(PoolMemberKindLikeCpp::GameObject, 4640131, 464)
        .expect("test pool relation");

    map.ensure_grid_loaded(&cell_from_world(1.0, 2.0));
    let mut gameobject = game_object_with_counter(4640131, 571, 7, false);
    let gameobject_guid = gameobject.world().guid();
    gameobject.set_go_type(GAMEOBJECT_TYPE_GENERIC_LIKE_CPP as u8);
    gameobject.world_mut().object_mut().set_entry(190_131);
    gameobject.set_spawn_id(4640131);
    gameobject.set_represented_gameobject_data_present_like_cpp(true);
    gameobject.set_respawn_compatibility_mode(true);
    gameobject.set_created_by(guid(HighGuid::Player, 4640199));
    gameobject.set_respawn_time(0);
    gameobject.set_loot_state(LootState::JustDeactivated, None);
    map.add_map_object_record_to_map_like_cpp(
        MapObjectRecord::new_game_object(gameobject).unwrap(),
    )
    .unwrap();
    map.pool_data_mut_like_cpp()
        .add_spawn_like_cpp(SpawnObjectType::GameObject, 4640131, 464)
        .expect("trigger spawned in pool");

    let replacement_guid = guid(HighGuid::GameObject, 4640132);
    let mut loader_calls = 0usize;
    let outcome = map.update_game_object_with_pool_update_loaded_grid_records_like_cpp(
        gameobject_guid,
        1,
        1_000,
        &store,
        &pool_mgr,
        |_, object_type, spawn_id| {
            loader_calls += 1;
            assert_eq!(object_type, SpawnObjectType::GameObject);
            assert_eq!(spawn_id, 4640132);
            Some(LoadedGridRespawnRecordsLikeCpp::primary_only(
                MapObjectRecord::new_game_object(test_gameobject_for_spawn(spawn_id, 4640132))
                    .unwrap(),
            ))
        },
    );

    assert_eq!(
        outcome.status,
        GameObjectUpdateStatusLikeCpp::DespawnPoolUpdated
    );
    assert_eq!(loader_calls, 1);
    assert!(outcome.summoned_expired_delete);
    assert!(outcome.remove_list.is_none());
    assert_eq!(map.objects_to_remove_count_like_cpp(), 0);
    assert!(map.map_object_record(gameobject_guid).is_none());
    assert!(map.map_object_record(replacement_guid).is_some());
    assert!(
        map.pool_data_like_cpp()
            .is_spawned_gameobject_like_cpp(4640132)
    );
}
#[test]
fn gameobject_update_generic_spawned_default_compat_saves_db_only_and_visibility_like_cpp() {
    let mut map = test_map();
    let mut gameobject = game_object_with_counter(4640201, 571, 7, false);
    let gameobject_guid = gameobject.world().guid();
    gameobject.set_go_type(GAMEOBJECT_TYPE_GENERIC_LIKE_CPP as u8);
    gameobject.world_mut().object_mut().set_entry(190002);
    gameobject.set_spawn_id(4640201);
    gameobject.set_spawned_by_default(true);
    gameobject.set_represented_gameobject_data_present_like_cpp(true);
    gameobject.set_respawn_compatibility_mode(true);
    gameobject.set_respawn_delay_time(30);
    gameobject.set_loot_state(LootState::JustDeactivated, None);

    map.add_map_object_record_to_map_like_cpp(
        MapObjectRecord::new_game_object(gameobject).unwrap(),
    )
    .unwrap();

    let outcome = map.update_game_object_like_cpp(gameobject_guid, 1, 2_000);
    let canonical = map
        .map_object_record(gameobject_guid)
        .and_then(MapObjectRecord::game_object)
        .unwrap();

    assert_eq!(outcome.status, GameObjectUpdateStatusLikeCpp::Updated);
    assert!(outcome.generic_not_ready);
    assert_eq!(outcome.generic_respawn_scheduled_time, Some(2_030));
    assert!(outcome.generic_spawned_by_default_branch);
    assert_eq!(outcome.generic_respawn_timer_add, None);
    assert!(outcome.generic_respawn_compatibility_db_only_represented);
    assert!(outcome.generic_visibility_on_destroy_represented);
    assert!(outcome.remove_list.is_none());
    assert_eq!(canonical.respawn_time(), 2_030);
    assert!(
        map.get_respawn_info_like_cpp(SpawnObjectType::GameObject, 4640201)
            .is_none()
    );
    assert_eq!(map.objects_to_remove_count_like_cpp(), 0);
}
#[test]
fn gameobject_visibility_on_destroy_update_summary_carries_guids_without_truncation_like_cpp() {
    let mut map = test_map();
    let mut expected_guids = Vec::new();

    for offset in 0..300 {
        let counter = 5_050_101_i64 + i64::from(offset);
        let mut gameobject = game_object_with_counter(counter, 571, 7, false);
        let gameobject_guid = gameobject.world().guid();
        gameobject.set_go_type(GAMEOBJECT_TYPE_GENERIC_LIKE_CPP as u8);
        gameobject.world_mut().object_mut().set_entry(190_505);
        gameobject.set_spawn_id(u64::try_from(counter).unwrap());
        gameobject.set_spawned_by_default(true);
        gameobject.set_represented_gameobject_data_present_like_cpp(true);
        gameobject.set_respawn_compatibility_mode(true);
        gameobject.set_respawn_delay_time(30);
        gameobject.set_loot_state(LootState::JustDeactivated, None);

        expected_guids.push(gameobject_guid);
        map.add_map_object_record_to_map_like_cpp(
            MapObjectRecord::new_game_object(gameobject).unwrap(),
        )
        .unwrap();
    }

    let summary = map.update_game_objects_like_cpp(1, 2_000);

    assert_eq!(summary.generic_visibility_on_destroy_represented, 300);
    assert_eq!(
        summary.generic_respawn_compatibility_db_only_represented,
        300
    );
    assert_eq!(summary.respawn_db_saves.len(), 300);
    assert_eq!(
        summary.respawn_db_saves[256].object_type,
        SpawnObjectType::GameObject
    );
    assert_eq!(summary.respawn_db_saves[256].respawn_time, 2_030);
    let carried_guids = summary.generic_visibility_on_destroy_guids.as_slice();
    assert_eq!(carried_guids.len(), 300);
    assert!(carried_guids.contains(&expected_guids[256]));
    assert!(carried_guids.contains(&expected_guids[299]));
    let carried_set = carried_guids
        .iter()
        .copied()
        .collect::<std::collections::HashSet<_>>();
    let expected_set = expected_guids
        .iter()
        .copied()
        .collect::<std::collections::HashSet<_>>();
    assert_eq!(carried_set, expected_set);
    assert_eq!(
        summary.generic_respawn_compatibility_db_only_represented,
        300
    );
    assert_eq!(summary.despawn_remove_queued, 0);
    for guid in expected_guids {
        assert!(map.map_object_record(guid).is_some());
    }
}
#[test]
fn gameobject_update_generic_spawned_default_noncompat_missing_godata_does_not_insert_respawn_like_cpp()
 {
    let mut map = test_map();
    let mut gameobject = game_object_with_counter(4640251, 571, 7, false);
    let gameobject_guid = gameobject.world().guid();
    gameobject.set_go_type(GAMEOBJECT_TYPE_GENERIC_LIKE_CPP as u8);
    gameobject.world_mut().object_mut().set_entry(190003);
    gameobject.set_spawn_id(4640251);
    gameobject.set_spawned_by_default(true);
    gameobject.set_respawn_compatibility_mode(false);
    gameobject.set_respawn_delay_time(25);
    gameobject.set_loot_state(LootState::JustDeactivated, None);

    map.add_map_object_record_to_map_like_cpp(
        MapObjectRecord::new_game_object(gameobject).unwrap(),
    )
    .unwrap();

    let outcome = map.update_game_object_like_cpp(gameobject_guid, 1, 2_500);
    let canonical = map
        .map_object_record(gameobject_guid)
        .and_then(MapObjectRecord::game_object)
        .unwrap();

    assert_eq!(
        outcome.status,
        GameObjectUpdateStatusLikeCpp::DespawnRemoveQueued
    );
    assert!(outcome.generic_not_ready);
    assert_eq!(outcome.generic_respawn_scheduled_time, Some(2_525));
    assert!(outcome.generic_spawned_by_default_branch);
    assert_eq!(outcome.generic_respawn_timer_add, None);
    assert!(outcome.generic_respawn_save_missing_gameobject_data);
    assert!(!outcome.generic_respawn_save_missing_spawn_id);
    assert!(!outcome.generic_respawn_compatibility_db_only_represented);
    assert!(!outcome.generic_visibility_on_destroy_represented);
    assert!(outcome.remove_list.is_some());
    assert_eq!(canonical.respawn_time(), 2_525);
    assert!(
        map.get_respawn_info_like_cpp(SpawnObjectType::GameObject, 4640251)
            .is_none()
    );
    assert_eq!(map.objects_to_remove_count_like_cpp(), 1);
}
#[test]
fn gameobject_update_generic_temporary_noncompat_spawn_id_visibility_no_remove_like_cpp() {
    let mut map = test_map();
    let mut gameobject = game_object_with_counter(4640301, 571, 7, false);
    let gameobject_guid = gameobject.world().guid();
    gameobject.set_go_type(GAMEOBJECT_TYPE_GENERIC_LIKE_CPP as u8);
    gameobject.set_spawn_id(4640301);
    gameobject.set_spawned_by_default(false);
    gameobject.set_respawn_compatibility_mode(false);
    gameobject.set_respawn_delay_time(60);
    gameobject.set_respawn_time(999);
    gameobject.set_loot_state(LootState::JustDeactivated, None);

    map.add_map_object_record_to_map_like_cpp(
        MapObjectRecord::new_game_object(gameobject).unwrap(),
    )
    .unwrap();

    let outcome = map.update_game_object_like_cpp(gameobject_guid, 1, 3_000);
    let canonical = map
        .map_object_record(gameobject_guid)
        .and_then(MapObjectRecord::game_object)
        .unwrap();

    assert_eq!(outcome.status, GameObjectUpdateStatusLikeCpp::Updated);
    assert!(outcome.generic_not_ready);
    assert_eq!(outcome.generic_respawn_scheduled_time, None);
    assert!(!outcome.generic_spawned_by_default_branch);
    assert!(outcome.generic_temporary_respawn_zeroed);
    assert_eq!(outcome.generic_respawn_timer_add, None);
    assert!(outcome.generic_visibility_on_destroy_represented);
    assert!(outcome.remove_list.is_none());
    assert_eq!(canonical.respawn_time(), 0);
    assert!(
        map.get_respawn_info_like_cpp(SpawnObjectType::GameObject, 4640301)
            .is_none()
    );
    assert_eq!(map.objects_to_remove_count_like_cpp(), 0);
}
#[test]
fn gameobject_update_generic_temporary_zero_spawn_id_deletes_remove_like_cpp() {
    let mut map = test_map();
    let mut gameobject = game_object_with_counter(4640351, 571, 7, false);
    let gameobject_guid = gameobject.world().guid();
    gameobject.set_go_type(GAMEOBJECT_TYPE_GENERIC_LIKE_CPP as u8);
    gameobject.set_spawn_id(0);
    gameobject.set_spawned_by_default(false);
    gameobject.set_respawn_compatibility_mode(false);
    gameobject.set_respawn_delay_time(60);
    gameobject.set_respawn_time(999);
    gameobject.set_loot_state(LootState::JustDeactivated, None);

    map.add_map_object_record_to_map_like_cpp(
        MapObjectRecord::new_game_object(gameobject).unwrap(),
    )
    .unwrap();

    let outcome = map.update_game_object_like_cpp(gameobject_guid, 1, 3_100);
    let canonical = map
        .map_object_record(gameobject_guid)
        .and_then(MapObjectRecord::game_object)
        .unwrap();

    assert_eq!(
        outcome.status,
        GameObjectUpdateStatusLikeCpp::DespawnRemoveQueued
    );
    assert!(outcome.generic_not_ready);
    assert!(outcome.generic_temporary_respawn_zeroed);
    assert!(!outcome.generic_visibility_on_destroy_represented);
    assert!(outcome.remove_list.is_some());
    assert_eq!(canonical.respawn_time(), 0);
    assert_eq!(map.objects_to_remove_count_like_cpp(), 1);
}
#[test]
fn gameobject_update_generic_zero_respawn_sets_not_ready_without_remove_like_cpp() {
    let mut map = test_map();
    let mut gameobject = game_object_with_counter(4630101, 571, 7, false);
    let gameobject_guid = gameobject.world().guid();
    gameobject.set_go_type(GAMEOBJECT_TYPE_GENERIC_LIKE_CPP as u8);
    gameobject.set_respawn_delay_time(0);
    gameobject.set_loot_state(LootState::JustDeactivated, None);

    map.add_map_object_record_to_map_like_cpp(
        MapObjectRecord::new_game_object(gameobject).unwrap(),
    )
    .unwrap();

    let outcome = map.update_game_object_like_cpp(gameobject_guid, 1, 1_000);
    let canonical = map
        .map_object_record(gameobject_guid)
        .and_then(MapObjectRecord::game_object)
        .unwrap();

    assert_eq!(outcome.status, GameObjectUpdateStatusLikeCpp::Updated);
    assert!(outcome.loot_cleared);
    assert!(outcome.generic_not_ready);
    assert!(outcome.generic_zero_respawn_delay_return);
    assert!(!outcome.generic_visual_despawn_represented);
    assert!(!outcome.generic_flags_restored_represented);
    assert_eq!(canonical.loot_state(), LootState::NotReady);
    assert_eq!(map.objects_to_remove_count_like_cpp(), 0);
}
#[test]
fn gameobject_update_generic_chest_consumable_visual_despawn_restores_flags_like_cpp() {
    let mut map = test_map();
    let mut gameobject = game_object_with_counter(4630201, 571, 7, false);
    let gameobject_guid = gameobject.world().guid();
    gameobject.set_go_type(GAMEOBJECT_TYPE_CHEST as u8);
    gameobject.set_represented_chest_loot_source_like_cpp(Some(GameObjectLootSource {
        chest_consumable: true,
        ..GameObjectLootSource::default()
    }));
    gameobject.set_represented_baseline_flags_like_cpp(Some(0x10));
    gameobject.set_flags(0x90);
    gameobject.set_respawn_delay_time(0);
    gameobject.set_loot_state(LootState::JustDeactivated, None);

    map.add_map_object_record_to_map_like_cpp(
        MapObjectRecord::new_game_object(gameobject).unwrap(),
    )
    .unwrap();

    let outcome = map.update_game_object_like_cpp(gameobject_guid, 1, 1_000);
    let canonical = map
        .map_object_record(gameobject_guid)
        .and_then(MapObjectRecord::game_object)
        .unwrap();

    assert!(outcome.generic_not_ready);
    assert!(outcome.generic_visual_despawn_represented);
    assert!(outcome.generic_flags_restored_represented);
    assert!(!outcome.generic_despawn_at_action_source_missing);
    assert_eq!(canonical.data().flags, 0x10);
    assert_eq!(canonical.loot_state(), LootState::NotReady);
    assert_eq!(map.objects_to_remove_count_like_cpp(), 0);
}
#[test]
fn gameobject_update_generic_anim_progress_visual_despawn_without_despawn_source_like_cpp() {
    let mut map = test_map();
    let mut gameobject = game_object_with_counter(4630301, 571, 7, false);
    let gameobject_guid = gameobject.world().guid();
    gameobject.set_go_type(GAMEOBJECT_TYPE_GENERIC_LIKE_CPP as u8);
    gameobject.set_go_anim_progress_like_cpp(1);
    gameobject.set_represented_baseline_flags_like_cpp(Some(0x04));
    gameobject.set_flags(0x84);
    gameobject.set_respawn_delay_time(0);
    gameobject.set_loot_state(LootState::JustDeactivated, None);

    map.add_map_object_record_to_map_like_cpp(
        MapObjectRecord::new_game_object(gameobject).unwrap(),
    )
    .unwrap();

    let outcome = map.update_game_object_like_cpp(gameobject_guid, 1, 1_000);
    let canonical = map
        .map_object_record(gameobject_guid)
        .and_then(MapObjectRecord::game_object)
        .unwrap();

    assert!(outcome.generic_not_ready);
    assert!(outcome.generic_visual_despawn_represented);
    assert!(outcome.generic_flags_restored_represented);
    assert!(!outcome.generic_despawn_at_action_source_missing);
    assert_eq!(canonical.data().flags, 0x04);
    assert_eq!(canonical.loot_state(), LootState::NotReady);
}
#[test]
fn gameobject_update_generic_chest_missing_source_does_not_assume_despawn_at_action_like_cpp() {
    let mut map = test_map();
    let mut gameobject = game_object_with_counter(4630401, 571, 7, false);
    let gameobject_guid = gameobject.world().guid();
    gameobject.set_go_type(GAMEOBJECT_TYPE_CHEST as u8);
    gameobject.set_represented_chest_loot_source_like_cpp(None);
    gameobject.set_respawn_delay_time(0);
    gameobject.set_loot_state(LootState::JustDeactivated, None);

    map.add_map_object_record_to_map_like_cpp(
        MapObjectRecord::new_game_object(gameobject).unwrap(),
    )
    .unwrap();

    let outcome = map.update_game_object_like_cpp(gameobject_guid, 1, 1_000);
    let canonical = map
        .map_object_record(gameobject_guid)
        .and_then(MapObjectRecord::game_object)
        .unwrap();

    assert!(outcome.non_consumed_source_missing);
    assert!(outcome.generic_not_ready);
    assert!(outcome.generic_despawn_at_action_source_missing);
    assert!(!outcome.generic_visual_despawn_represented);
    assert!(!outcome.generic_flags_restored_represented);
    assert_eq!(canonical.loot_state(), LootState::NotReady);
}
#[test]
fn gameobject_update_summary_counts_generic_branch_like_cpp() {
    let mut map = test_map();
    let mut gameobject = game_object_with_counter(4630501, 571, 7, false);
    gameobject.set_go_type(GAMEOBJECT_TYPE_GENERIC_LIKE_CPP as u8);
    gameobject.set_go_anim_progress_like_cpp(3);
    gameobject.set_represented_baseline_flags_like_cpp(Some(0x08));
    gameobject.set_flags(0x88);
    gameobject.set_respawn_delay_time(0);
    gameobject.set_loot_state(LootState::JustDeactivated, None);
    let gameobject_guid = gameobject.world().guid();

    map.add_map_object_record_to_map_like_cpp(
        MapObjectRecord::new_game_object(gameobject).unwrap(),
    )
    .unwrap();

    let summary = map.update_game_objects_like_cpp(1, 1_000);

    assert_eq!(summary.generic_not_ready, 1);
    assert_eq!(summary.generic_visual_despawn_represented, 1);
    assert_eq!(
        summary.generic_visual_despawn_guids.as_slice(),
        &[gameobject_guid]
    );
    assert_eq!(summary.generic_flags_restored_represented, 1);
    assert_eq!(summary.generic_zero_respawn_delay_returns, 1);
    assert_eq!(summary.despawn_remove_queued, 0);
}
#[test]
fn gameobject_visual_despawn_summary_guids_are_not_truncated_like_cpp() {
    let mut map = test_map();
    let mut expected_guids = Vec::new();
    for index in 0..300 {
        let mut gameobject = game_object_with_counter(4630601 + index, 571, 7, false);
        gameobject.set_go_type(GAMEOBJECT_TYPE_GENERIC_LIKE_CPP as u8);
        gameobject.set_go_anim_progress_like_cpp(1);
        gameobject.set_represented_baseline_flags_like_cpp(Some(0x08));
        gameobject.set_flags(0x88);
        gameobject.set_respawn_delay_time(0);
        gameobject.set_loot_state(LootState::JustDeactivated, None);
        expected_guids.push(gameobject.world().guid());
        map.add_map_object_record_to_map_like_cpp(
            MapObjectRecord::new_game_object(gameobject).unwrap(),
        )
        .unwrap();
    }

    let summary = map.update_game_objects_like_cpp(1, 1_000);

    assert_eq!(summary.generic_visual_despawn_represented, 300);
    assert_eq!(summary.generic_visual_despawn_guids.as_slice().len(), 300);
    let mut actual_guids = summary.generic_visual_despawn_guids.as_slice().to_vec();
    actual_guids.sort_by_key(|guid| guid.counter());
    expected_guids.sort_by_key(|guid| guid.counter());
    assert_eq!(actual_guids, expected_guids);
}
#[test]
fn gameobject_update_summary_counts_summoned_expired_delete_like_cpp() {
    let mut map = test_map();
    let mut owner = game_object_with_counter(4620701, 571, 7, false);
    let owner_guid = owner.world().guid();
    owner.set_go_type(GAMEOBJECT_TYPE_NEW_FLAG as u8);
    let mut drop = game_object_with_counter(4620702, 571, 7, false);
    drop.set_go_type(GAMEOBJECT_TYPE_NEW_FLAG_DROP as u8);
    drop.set_created_by(owner_guid);
    drop.set_respawn_time(0);
    drop.set_loot_state(LootState::JustDeactivated, None);

    map.add_map_object_record_to_map_like_cpp(MapObjectRecord::new_game_object(owner).unwrap())
        .unwrap();
    map.add_map_object_record_to_map_like_cpp(MapObjectRecord::new_game_object(drop).unwrap())
        .unwrap();

    let summary = map.update_game_objects_like_cpp(1, 1_000);

    assert_eq!(summary.summoned_expired_deletes, 1);
    assert_eq!(summary.summoned_expired_respawn_time_zeroed, 1);
    assert_eq!(summary.summoned_expired_despawn_represented, 1);
    assert_eq!(summary.new_flag_drop_owner_in_base_commands_represented, 1);
    assert_eq!(summary.despawn_remove_queued, 1);
}
#[test]
fn gameobject_update_missing_template_source_does_not_assume_non_consumed_like_cpp() {
    let mut map = test_map();
    let mut gameobject = game_object_with_counter(4610601, 571, 7, false);
    let gameobject_guid = gameobject.world().guid();
    gameobject.set_go_type(GAMEOBJECT_TYPE_CHEST as u8);
    gameobject.set_represented_chest_loot_source_like_cpp(None);
    gameobject.set_loot_state(LootState::JustDeactivated, None);

    map.add_map_object_record_to_map_like_cpp(
        MapObjectRecord::new_game_object(gameobject).unwrap(),
    )
    .unwrap();

    let outcome = map.update_game_object_like_cpp(gameobject_guid, 1, 1_000);

    assert!(outcome.loot_cleared);
    assert!(!outcome.non_consumed_chest_or_goober_return);
    assert!(!outcome.non_consumed_set_ready);
    assert!(!outcome.non_consumed_restock_armed);
    assert!(outcome.non_consumed_source_missing);
}
#[test]
fn gameobject_update_despawn_requested_does_not_consume_just_deactivated_linked_trap_like_cpp() {
    let mut map = test_map();
    let mut owner = game_object_with_counter(4580501, 571, 7, false);
    let trap = game_object_with_counter(4580502, 571, 7, false);
    let owner_guid = owner.world().guid();
    let trap_guid = trap.world().guid();
    owner.set_loot_state(LootState::JustDeactivated, None);
    owner.set_linked_trap_like_cpp(trap_guid);
    owner.set_shared_loot_like_cpp(GameObjectOwnedLoot::new(5, 1));
    owner.set_personal_loot_like_cpp(
        guid(HighGuid::Player, 4590291),
        GameObjectOwnedLoot::new(6, 1),
    );
    let loot_authority = owner.loot_authority_like_cpp().clone();
    assert!(owner.add_unique_use_like_cpp(guid(HighGuid::Player, 4590292)));
    assert!(owner.schedule_despawn_or_unsummon_like_cpp(1, 0));

    map.add_map_object_record_to_map_like_cpp(MapObjectRecord::new_game_object(trap).unwrap())
        .unwrap();
    map.add_map_object_record_to_map_like_cpp(MapObjectRecord::new_game_object(owner).unwrap())
        .unwrap();

    let outcome = map.update_game_object_like_cpp(owner_guid, 1, 1_000);

    assert_eq!(
        outcome.status,
        GameObjectUpdateStatusLikeCpp::DespawnRemoveQueued
    );
    assert_eq!(outcome.linked_trap_guid, None);
    assert!(!outcome.linked_trap_removed);
    assert!(!outcome.linked_trap_missing_or_self);
    assert!(!outcome.loot_cleared);
    assert_eq!(map.objects_to_remove_count_like_cpp(), 1);
    let owner_after = map
        .map_object_record(owner_guid)
        .and_then(MapObjectRecord::game_object)
        .unwrap();
    assert_eq!(owner_after.loot_state(), LootState::NotReady);
    assert!(owner_after.shared_loot_like_cpp().is_some());
    assert_eq!(owner_after.personal_loot_count_like_cpp(), 1);
    assert_eq!(owner_after.unique_user_count_like_cpp(), 1);
    assert_eq!(owner_after.use_times(), 1);
    assert!(loot_authority.is_retired_like_cpp());
    assert!(map.map_object_record(trap_guid).is_some());
}
#[test]
fn gameobject_update_non_just_deactivated_keeps_linked_trap_like_cpp() {
    let mut map = test_map();
    let mut owner = game_object_with_counter(4580201, 571, 7, false);
    let trap = game_object_with_counter(4580202, 571, 7, false);
    let owner_guid = owner.world().guid();
    let trap_guid = trap.world().guid();
    owner.set_loot_state(LootState::Ready, None);
    owner.set_linked_trap_like_cpp(trap_guid);
    owner.set_shared_loot_like_cpp(GameObjectOwnedLoot::new(9, 1));
    owner.set_personal_loot_like_cpp(
        guid(HighGuid::Player, 4590391),
        GameObjectOwnedLoot::new(10, 1),
    );
    assert!(owner.add_unique_use_like_cpp(guid(HighGuid::Player, 4590392)));

    map.add_map_object_record_to_map_like_cpp(MapObjectRecord::new_game_object(trap).unwrap())
        .unwrap();
    map.add_map_object_record_to_map_like_cpp(MapObjectRecord::new_game_object(owner).unwrap())
        .unwrap();

    let outcome = map.update_game_object_like_cpp(owner_guid, 1, 1_000);

    assert_eq!(outcome.status, GameObjectUpdateStatusLikeCpp::Updated);
    assert_eq!(outcome.linked_trap_guid, None);
    assert!(!outcome.linked_trap_removed);
    assert!(!outcome.linked_trap_missing_or_self);
    assert!(!outcome.loot_cleared);
    let owner_after = map
        .map_object_record(owner_guid)
        .and_then(MapObjectRecord::game_object)
        .unwrap();
    assert_eq!(owner_after.loot_state(), LootState::Ready);
    assert!(owner_after.shared_loot_like_cpp().is_some());
    assert_eq!(owner_after.personal_loot_count_like_cpp(), 1);
    assert_eq!(owner_after.unique_user_count_like_cpp(), 1);
    assert_eq!(owner_after.use_times(), 1);
    assert!(map.map_object_record(owner_guid).is_some());
    assert!(map.map_object_record(trap_guid).is_some());
}
#[test]
fn gameobject_update_not_in_world_does_not_clear_owned_loot_like_cpp() {
    let mut map = test_map();
    let mut gameobject = game_object_with_counter(4590401, 571, 7, false);
    let gameobject_guid = gameobject.world().guid();
    gameobject.set_loot_state(LootState::JustDeactivated, None);
    gameobject.set_shared_loot_like_cpp(GameObjectOwnedLoot::new(12, 1));
    gameobject.set_personal_loot_like_cpp(
        guid(HighGuid::Player, 4590491),
        GameObjectOwnedLoot::new(13, 1),
    );
    assert!(gameobject.add_unique_use_like_cpp(guid(HighGuid::Player, 4590492)));

    map.insert_map_object_record(MapObjectRecord::new_game_object(gameobject).unwrap())
        .unwrap();

    let outcome = map.update_game_object_like_cpp(gameobject_guid, 1, 1_000);
    let canonical = map
        .map_object_record(gameobject_guid)
        .and_then(MapObjectRecord::game_object)
        .unwrap();

    assert_eq!(outcome.status, GameObjectUpdateStatusLikeCpp::NotInWorld);
    assert!(!outcome.loot_cleared);
    assert_eq!(canonical.loot_state(), LootState::JustDeactivated);
    assert!(canonical.shared_loot_like_cpp().is_some());
    assert_eq!(canonical.personal_loot_count_like_cpp(), 1);
    assert_eq!(canonical.unique_user_count_like_cpp(), 1);
    assert_eq!(canonical.use_times(), 1);
}
