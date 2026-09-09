//! Persistence scenarios for [`super`].
//!
//! Split out of map_tests.rs under #628; assertions and
//! registrations are unchanged and shared fixtures stay in the parent module.

use super::*;

#[test]
fn active_non_player_zero_spawn_mutates_set_without_unload_lock_like_cpp() {
    let mut map = test_map();
    let mut gameobject = test_gameobject_for_spawn(0, 4850301);
    let guid = gameobject.world().guid();
    gameobject.world_mut().set_active(true);
    gameobject.world_mut().object_mut().remove_from_world();

    let add = map
        .add_map_object_record_to_map_like_cpp(
            MapObjectRecord::new_game_object(gameobject).unwrap(),
        )
        .unwrap();
    let add_active = add.add_to_map_tail.unwrap().add_to_active.unwrap();
    assert!(add_active.inserted_in_active_set);
    assert!(add_active.spawn_id_zero_or_unsupported);
    assert!(add_active.unload_lock.is_none());
    assert!(map.is_active_non_player_like_cpp(guid));

    let remove = map.remove_from_map_like_cpp(guid, true).unwrap();
    let remove_active = remove.remove_from_active.unwrap();
    assert!(remove_active.removed_from_active_set);
    assert!(remove_active.spawn_id_zero_or_unsupported);
    assert!(remove_active.unload_lock.is_none());
    assert!(!map.is_active_non_player_like_cpp(guid));
}
#[test]
fn map_grid_state_delayed_helper_removal_lock_and_active_near_defer_unload_like_cpp() {
    let mut locked_map = test_map();
    let locked_position = Position::xyz(3_100.0, 3_100.0, 0.0);
    assert!(locked_map.load_grid(locked_position.x, locked_position.y));
    let locked_cell = Cell::from_world(locked_position.x, locked_position.y);
    let locked_coord = GridCoord::new(locked_cell.grid_x(), locked_cell.grid_y());
    let locked_grid = locked_map.get_ngrid_mut(locked_coord).unwrap();
    locked_grid.set_state(GridStateKind::Removal);
    locked_grid.info_mut().reset_time_tracker(1);
    locked_grid.info_mut().set_unload_explicit_lock(true);

    let locked_summary = locked_map.update_loaded_grid_states_like_cpp(1);

    assert_eq!(locked_summary.visited, 1);
    assert_eq!(locked_summary.updated, 1);
    assert_eq!(locked_summary.unloaded, 0);
    assert_eq!(locked_summary.removal_deferred_or_reset, 1);
    assert_eq!(
        locked_map.get_ngrid(locked_coord).unwrap().state(),
        GridStateKind::Removal
    );

    let mut active_near_map = test_map();
    let active_position = Position::xyz(3_200.0, 3_200.0, 0.0);
    assert!(active_near_map.load_grid(active_position.x, active_position.y));
    let active_cell = Cell::from_world(active_position.x, active_position.y);
    let active_coord = GridCoord::new(active_cell.grid_x(), active_cell.grid_y());
    let active_grid = active_near_map.get_ngrid_mut(active_coord).unwrap();
    active_grid.set_state(GridStateKind::Removal);
    active_grid.info_mut().reset_time_tracker(1);
    active_near_map.mark_active_cell(active_cell.cell_coord());

    let active_summary = active_near_map.update_loaded_grid_states_like_cpp(1);

    assert_eq!(active_summary.visited, 1);
    assert_eq!(active_summary.updated, 1);
    assert_eq!(active_summary.unloaded, 0);
    assert_eq!(active_summary.removal_deferred_or_reset, 1);
    assert_eq!(
        active_near_map.get_ngrid(active_coord).unwrap().state(),
        GridStateKind::Removal
    );
}
#[test]
fn grid_unload_map_store_missing_and_kind_mismatch_are_best_effort() {
    let mut map = test_map();
    let go_guid = guid(HighGuid::GameObject, 3713);
    let gameobject = test_gameobject_for_spawn(373, 3713);
    map.insert_map_object_record(MapObjectRecord::new_game_object(gameobject).unwrap())
        .unwrap();

    assert_eq!(
        apply_grid_unload_action(
            &mut map,
            GridUnloadAction::CreatureRespawnRelocation(go_guid),
        ),
        GridUnloadApplyOutcome::MissingEntity
    );
    assert_eq!(
        apply_grid_unload_action(
            &mut map,
            GridUnloadAction::CreatureRespawnRelocation(guid(HighGuid::Creature, 3714)),
        ),
        GridUnloadApplyOutcome::MissingEntity
    );

    let gameobject = map
        .map_object_record(go_guid)
        .unwrap()
        .game_object()
        .unwrap();
    assert!(!gameobject.grid_unload_respawn_relocation_requested());
    assert_eq!(gameobject.cleanup_before_delete_count(), 0);
    assert!(!gameobject.grid_unload_delete_requested());
}
#[test]
fn loaded_grid_area_trigger_records_callback_adds_map_owned_record_like_cpp() {
    let mut map = test_map();
    let group = SpawnGroupTemplateData::legacy_group();
    let spawn = spawn_data(SpawnObjectType::AreaTrigger, 8801, group);
    let mut store = SpawnStore::new();
    store.add_area_trigger_spawn(&spawn);
    map.ensure_grid_loaded(&cell_from_world(0.0, 0.0));

    let mut calls = Vec::new();
    let summary = map.load_loaded_grid_area_trigger_records_like_cpp(
        GridCoord::new(32, 32),
        &store,
        |_, object_type, spawn_id| {
            calls.push((object_type, spawn_id));
            Some(LoadedGridRespawnRecordsLikeCpp::primary_only(
                MapObjectRecord::new_area_trigger(test_area_trigger_for_spawn(spawn_id, 880101))
                    .unwrap(),
            ))
        },
    );

    assert_eq!(calls, vec![(SpawnObjectType::AreaTrigger, 8801)]);
    assert!(!summary.grid_not_loaded);
    assert_eq!(summary.metadata_entries, 1);
    assert_eq!(summary.loaded_grid_primary_records.len(), 1);
    assert_eq!(summary.add_to_map_errors, 0);
    assert!(map.get_area_trigger_by_spawn_id_like_cpp(8801).is_some());
    assert_eq!(map.area_trigger_spawn_id_store_count_like_cpp(8801), 1);
}
#[test]
fn loaded_grid_area_trigger_records_respect_spawn_grid_load_state_like_cpp() {
    let mut map = test_map();
    let manual = spawn_group(90, SpawnGroupFlags::MANUAL_SPAWN);
    let spawn = spawn_data(SpawnObjectType::AreaTrigger, 8802, manual);
    let mut store = SpawnStore::new();
    store.add_area_trigger_spawn(&spawn);
    map.ensure_grid_loaded(&cell_from_world(0.0, 0.0));

    let mut callback_calls = 0;
    let summary = map.load_loaded_grid_area_trigger_records_like_cpp(
        GridCoord::new(32, 32),
        &store,
        |_, _, _| {
            callback_calls += 1;
            None
        },
    );

    assert_eq!(callback_calls, 0);
    assert_eq!(summary.metadata_entries, 0);
    assert_eq!(summary.skipped_should_not_spawn, 1);
    assert!(summary.loaded_grid_primary_records.is_empty());
    assert!(map.get_area_trigger_by_spawn_id_like_cpp(8802).is_none());
}
#[test]
fn loaded_grid_area_trigger_records_skip_already_loaded_spawn_like_cpp() {
    let mut map = test_map();
    let group = SpawnGroupTemplateData::legacy_group();
    let spawn = spawn_data(SpawnObjectType::AreaTrigger, 8803, group);
    let mut store = SpawnStore::new();
    store.add_area_trigger_spawn(&spawn);
    map.ensure_grid_loaded(&cell_from_world(0.0, 0.0));
    map.add_map_object_record_to_map_like_cpp(
        MapObjectRecord::new_area_trigger(test_area_trigger_for_spawn(8803, 880301)).unwrap(),
    )
    .unwrap();

    let mut callback_calls = 0;
    let summary = map.load_loaded_grid_area_trigger_records_like_cpp(
        GridCoord::new(32, 32),
        &store,
        |_, _, _| {
            callback_calls += 1;
            Some(LoadedGridRespawnRecordsLikeCpp::primary_only(
                MapObjectRecord::new_area_trigger(test_area_trigger_for_spawn(8803, 880302))
                    .unwrap(),
            ))
        },
    );

    assert_eq!(callback_calls, 0);
    assert_eq!(summary.skipped_already_loaded, 1);
    assert_eq!(summary.metadata_entries, 0);
    assert!(summary.loaded_grid_primary_records.is_empty());
    assert_eq!(map.area_trigger_spawn_id_store_count_like_cpp(8803), 1);
}
#[test]
fn grid_load_state_uses_map_pool_data_like_cpp() {
    let mut map = test_map();
    let mut store = SpawnStore::new();
    let active = spawn_group(14, SpawnGroupFlags::NONE);
    let mut creature_spawn = spawn_data(SpawnObjectType::Creature, 501, active.clone());
    creature_spawn.pool_id = 7;
    let mut gameobject_spawn = spawn_data(SpawnObjectType::GameObject, 502, active);
    gameobject_spawn.pool_id = 7;
    store.add_object_spawn(&creature_spawn, |_| false);
    store.add_object_spawn(&gameobject_spawn, |_| false);

    let grid_state = map.spawn_grid_load_state_like_cpp(&store);
    assert!(!grid_state.should_be_spawned_on_grid_load(SpawnObjectType::Creature, 501));
    assert!(!grid_state.should_be_spawned_on_grid_load(SpawnObjectType::GameObject, 502));

    assert_eq!(
        map.pool_data_mut_like_cpp()
            .add_spawn_like_cpp(SpawnObjectType::Creature, 501, 7),
        Ok(())
    );
    let grid_state = map.spawn_grid_load_state_like_cpp(&store);
    assert!(grid_state.should_be_spawned_on_grid_load(SpawnObjectType::Creature, 501));
    assert!(!grid_state.should_be_spawned_on_grid_load(SpawnObjectType::GameObject, 502));

    assert_eq!(
        map.pool_data_mut_like_cpp()
            .add_spawn_like_cpp(SpawnObjectType::GameObject, 502, 7),
        Ok(())
    );
    let grid_state = map.spawn_grid_load_state_like_cpp(&store);
    assert!(grid_state.should_be_spawned_on_grid_load(SpawnObjectType::Creature, 501));
    assert!(grid_state.should_be_spawned_on_grid_load(SpawnObjectType::GameObject, 502));

    assert_eq!(
        map.pool_data_mut_like_cpp()
            .remove_spawn_like_cpp(SpawnObjectType::Creature, 501, 7),
        Ok(())
    );
    let grid_state = map.spawn_grid_load_state_like_cpp(&store);
    assert!(!grid_state.should_be_spawned_on_grid_load(SpawnObjectType::Creature, 501));
    assert!(grid_state.should_be_spawned_on_grid_load(SpawnObjectType::GameObject, 502));
}
#[test]
fn process_respawns_delete_only_active_due_timer_loaded_grid_blocks_do_respawn_and_preserves_timer_like_cpp()
 {
    let mut map = test_map();
    let mut store = SpawnStore::new();
    let active = spawn_group(13, SpawnGroupFlags::NONE);
    store.add_object_spawn(&spawn_data(SpawnObjectType::Creature, 43, active), |_| {
        false
    });
    map.ensure_grid_loaded(&cell_from_grid_center(GridCoord::new(7, 0)));
    map.add_respawn_info_like_cpp(respawn_info(SpawnObjectType::Creature, 43, 100));

    let summary = map.process_due_respawns_spawn_group_delete_only_like_cpp(100, &store);

    assert_eq!(summary.deleted_inactive_spawn_group, 0);
    assert_eq!(summary.processed_unloaded_grid_respawns, 0);
    assert_eq!(summary.blocked_do_respawn_runtime, 1);
    assert_eq!(
        map.get_respawn_time_like_cpp(SpawnObjectType::Creature, 43),
        100
    );
}
#[test]
fn process_respawns_allowed_unloaded_grid_removes_timer_and_continues_like_cpp() {
    let mut map = test_map();
    let mut store = SpawnStore::new();
    let active = spawn_group(16, SpawnGroupFlags::NONE);
    store.add_object_spawn(
        &spawn_data(SpawnObjectType::Creature, 47, active.clone()),
        |_| false,
    );
    store.add_object_spawn(&spawn_data(SpawnObjectType::GameObject, 48, active), |_| {
        false
    });
    map.add_respawn_info_like_cpp(respawn_info(SpawnObjectType::Creature, 47, 90));
    map.add_respawn_info_like_cpp(RespawnInfoLikeCpp {
        object_type: SpawnObjectType::GameObject,
        spawn_id: 48,
        entry: 42,
        respawn_time: 100,
        grid_id: 8,
    });

    let summary = map.process_due_respawns_spawn_group_delete_only_like_cpp(100, &store);

    assert_eq!(summary.processed_unloaded_grid_respawns, 2);
    assert_eq!(summary.blocked_do_respawn_runtime, 0);
    assert_eq!(summary.deleted_inactive_spawn_group, 0);
    assert_eq!(
        map.get_respawn_time_like_cpp(SpawnObjectType::Creature, 47),
        0
    );
    assert_eq!(
        map.get_respawn_time_like_cpp(SpawnObjectType::GameObject, 48),
        0
    );
    assert!(
        map.get_respawn_info_like_cpp(SpawnObjectType::Creature, 47)
            .is_none()
    );
    assert!(
        map.get_respawn_info_like_cpp(SpawnObjectType::GameObject, 48)
            .is_none()
    );
}
#[test]
fn process_respawns_loaded_grid_pre_add_records_are_best_effort_like_cpp() {
    let mut map = test_map();
    let mut store = SpawnStore::new();
    let active = spawn_group(409, SpawnGroupFlags::NONE);
    store.add_object_spawn(
        &spawn_data(SpawnObjectType::GameObject, 40901, active),
        |_| false,
    );
    map.ensure_grid_loaded(&cell_from_grid_center(GridCoord::new(7, 0)));
    map.add_respawn_info_like_cpp(respawn_info(SpawnObjectType::GameObject, 40901, 100));
    let owner_guid = guid(HighGuid::GameObject, 4090101);
    let trap_guid = guid(HighGuid::GameObject, 4090102);
    let missing_trap_guid = guid(HighGuid::GameObject, 4090103);

    let summary = map.process_due_respawns_composite_loaded_grid_respawns_like_cpp(
        100,
        &store,
        &LinkedRespawnStoreLikeCpp::new(),
        &PoolMgrLikeCpp::new(),
        5,
        false,
        |_, _| false,
        |_, _| 0.0,
        |_candidates, count| (0..count).collect(),
        true,
        |_map, object_type, spawn_id| {
            assert_eq!(object_type, SpawnObjectType::GameObject);
            assert_eq!(spawn_id, 40901);
            let mut trap = test_gameobject_for_spawn(0, 4090102);
            trap.world_mut().object_mut().remove_from_world();
            let mut missing_trap = test_gameobject_for_spawn(0, 4090103);
            missing_trap.world_mut().object_mut().remove_from_world();
            missing_trap
                .world_mut()
                .relocate(Position::xyz(1_000_000.0, 1_000_000.0, 0.0));
            let mut owner = test_gameobject_for_spawn(40901, 4090101);
            owner.world_mut().object_mut().remove_from_world();
            owner.set_linked_trap_like_cpp(trap_guid);
            Some(LoadedGridRespawnRecordsLikeCpp {
                pre_add_records: vec![
                    MapObjectRecord::new_game_object(trap).unwrap(),
                    MapObjectRecord::new_game_object(missing_trap).unwrap(),
                ],
                primary_record: MapObjectRecord::new_game_object(owner).unwrap(),
            })
        },
    );

    assert_eq!(summary.executed_loaded_grid_respawns, 1);
    assert_eq!(summary.blocked_loaded_grid_respawn_add_to_map, 0);
    assert!(map.map_object_record(owner_guid).is_some());
    assert!(map.map_object_record(trap_guid).is_some());
    assert!(map.map_object_record(missing_trap_guid).is_none());

    map.remove_from_map_like_cpp(owner_guid, true).unwrap();
    assert!(map.map_object_record(owner_guid).is_none());
    assert!(map.map_object_record(trap_guid).is_some());
    assert_eq!(map.objects_to_remove_count_like_cpp(), 1);

    let remove_list = map.remove_all_objects_in_remove_list_like_cpp();
    assert_eq!(remove_list.processed, 1);
    assert_eq!(remove_list.removed, 1);
    assert!(map.map_object_record(trap_guid).is_none());
}
#[test]
fn process_respawns_loaded_grid_loader_none_removes_timer_and_continues_like_cpp() {
    let mut map = test_map();
    let mut store = SpawnStore::new();
    let active = spawn_group(399, SpawnGroupFlags::NONE);
    store.add_object_spawn(
        &spawn_data(SpawnObjectType::Creature, 39901, active.clone()),
        |_| false,
    );
    store.add_object_spawn(
        &spawn_data(SpawnObjectType::Creature, 39902, active),
        |_| false,
    );
    map.ensure_grid_loaded(&cell_from_grid_center(GridCoord::new(7, 0)));
    map.add_respawn_info_like_cpp(respawn_info(SpawnObjectType::Creature, 39901, 90));
    map.add_respawn_info_like_cpp(respawn_info(SpawnObjectType::Creature, 39902, 100));

    let summary = map.process_due_respawns_composite_loaded_grid_respawns_like_cpp(
        100,
        &store,
        &LinkedRespawnStoreLikeCpp::new(),
        &PoolMgrLikeCpp::new(),
        5,
        false,
        |_, _| false,
        |_, _| 0.0,
        |_candidates, count| (0..count).collect(),
        true,
        |_map, object_type, spawn_id| {
            assert_eq!(object_type, SpawnObjectType::Creature);
            if spawn_id == 39901 {
                None
            } else {
                Some(LoadedGridRespawnRecordsLikeCpp::primary_only(
                    MapObjectRecord::new_creature(test_creature_for_spawn(spawn_id, 3990201, true))
                        .unwrap(),
                ))
            }
        },
    );

    assert_eq!(summary.executed_loaded_grid_respawns, 1);
    assert_eq!(summary.blocked_loaded_grid_respawn_loads, 1);
    assert_eq!(summary.blocked_do_respawn_runtime, 1);
    assert_eq!(map.map_object_count(), 1);
    assert!(map.get_gameobject_by_spawn_id_like_cpp(39902).is_none());
    assert!(map.get_creature_by_spawn_id_like_cpp(39902).is_some());
    assert_eq!(
        map.get_respawn_time_like_cpp(SpawnObjectType::Creature, 39901),
        0
    );
    assert_eq!(
        map.get_respawn_time_like_cpp(SpawnObjectType::Creature, 39902),
        0
    );
}
#[test]
fn process_respawns_unloaded_grid_allowed_branch_does_not_call_loader_like_cpp() {
    let mut map = test_map();
    let mut store = SpawnStore::new();
    let active = spawn_group(400, SpawnGroupFlags::NONE);
    let mut far_spawn = spawn_data(SpawnObjectType::Creature, 40001, active);
    far_spawn.spawn_point = crate::spawn::SpawnPosition::new(1_000.0, 1_000.0, 0.0, 0.0);
    store.add_object_spawn(&far_spawn, |_| false);
    map.add_respawn_info_like_cpp(respawn_info(SpawnObjectType::Creature, 40001, 100));
    let mut loader_calls = 0;

    let summary = map.process_due_respawns_composite_loaded_grid_respawns_like_cpp(
        100,
        &store,
        &LinkedRespawnStoreLikeCpp::new(),
        &PoolMgrLikeCpp::new(),
        5,
        false,
        |_, _| false,
        |_, _| 0.0,
        |_candidates, count| (0..count).collect(),
        true,
        |_map, _object_type, _spawn_id| {
            loader_calls += 1;
            None
        },
    );

    assert_eq!(loader_calls, 0);
    assert_eq!(summary.processed_unloaded_grid_respawns, 1);
    assert_eq!(summary.executed_loaded_grid_respawns, 0);
    assert_eq!(
        map.get_respawn_time_like_cpp(SpawnObjectType::Creature, 40001),
        0
    );
}
#[test]
fn process_respawns_loaded_grid_add_to_map_failure_counts_and_removes_timer_like_cpp() {
    let mut map = test_map();
    let mut store = SpawnStore::new();
    let active = spawn_group(401, SpawnGroupFlags::NONE);
    store.add_object_spawn(
        &spawn_data(SpawnObjectType::Creature, 40101, active),
        |_| false,
    );
    map.ensure_grid_loaded(&cell_from_grid_center(GridCoord::new(7, 0)));
    map.add_respawn_info_like_cpp(respawn_info(SpawnObjectType::Creature, 40101, 100));
    let expected_guid = guid(HighGuid::Creature, 4010101);

    let summary = map.process_due_respawns_composite_loaded_grid_respawns_like_cpp(
        100,
        &store,
        &LinkedRespawnStoreLikeCpp::new(),
        &PoolMgrLikeCpp::new(),
        5,
        false,
        |_, _| false,
        |_, _| 0.0,
        |_candidates, count| (0..count).collect(),
        true,
        |_map, _object_type, _spawn_id| {
            let mut creature = test_creature_for_spawn(40101, 4010101, true);
            creature
                .unit_mut()
                .world_mut()
                .object_mut()
                .remove_from_world();
            creature
                .unit_mut()
                .world_mut()
                .relocate(Position::xyz(1_000_000.0, 1_000_000.0, 0.0));
            Some(LoadedGridRespawnRecordsLikeCpp::primary_only(
                MapObjectRecord::new_creature(creature).unwrap(),
            ))
        },
    );

    assert_eq!(summary.executed_loaded_grid_respawns, 0);
    assert_eq!(summary.blocked_loaded_grid_respawn_add_to_map, 1);
    assert_eq!(summary.blocked_loaded_grid_respawn_loads, 0);
    assert_eq!(
        map.get_respawn_time_like_cpp(SpawnObjectType::Creature, 40101),
        0
    );
    assert!(map.map_object_record(expected_guid).is_none());
    assert_eq!(map.creature_spawn_id_store_count_like_cpp(40101), 0);
}
#[test]
fn process_respawns_pool_loaded_grid_spawn_one_loader_adds_record_and_removes_trigger_timer_like_cpp()
 {
    let mut map = test_map();
    let mut store = SpawnStore::new();
    let active = spawn_group(526, SpawnGroupFlags::NONE);
    store.add_object_spawn(
        &spawn_data(SpawnObjectType::Creature, 52601, active.clone()),
        |_| false,
    );
    store.add_object_spawn(
        &spawn_data(SpawnObjectType::Creature, 52602, active),
        |_| false,
    );
    map.ensure_grid_loaded(&cell_from_world(0.0, 0.0));
    map.add_respawn_info_like_cpp(respawn_info(SpawnObjectType::Creature, 52601, 100));
    let mut pool_mgr = PoolMgrLikeCpp::new();
    pool_mgr.insert_template_like_cpp(526, PoolTemplateDataLikeCpp::new(1, 571));
    let mut group = PoolGroupLikeCpp::with_pool_id(PoolMemberKindLikeCpp::Creature, 526);
    group.add_entry_like_cpp(PoolObjectLikeCpp::new(52601, 0.0), 1);
    group.add_entry_like_cpp(PoolObjectLikeCpp::new(52602, 0.0), 1);
    pool_mgr
        .insert_or_replace_group_like_cpp(PoolMemberKindLikeCpp::Creature, 526, group)
        .expect("test pool group");
    pool_mgr
        .register_spawn_pool_relation_like_cpp(PoolMemberKindLikeCpp::Creature, 52601, 526)
        .expect("test spawn pool relation");
    let expected_guid = guid(HighGuid::Creature, 5260201);
    let mut loader_calls = 0;

    let summary = map.process_due_respawns_composite_loaded_grid_respawns_like_cpp(
        100,
        &store,
        &LinkedRespawnStoreLikeCpp::new(),
        &pool_mgr,
        5,
        false,
        |_, _| false,
        |_, _| 0.0,
        |_candidates, _count| vec![1],
        true,
        |_map, object_type, spawn_id| {
            loader_calls += 1;
            assert_eq!(object_type, SpawnObjectType::Creature);
            assert_eq!(spawn_id, 52602);
            let mut creature = test_creature_for_spawn(52602, 5260201, true);
            creature
                .unit_mut()
                .world_mut()
                .object_mut()
                .remove_from_world();
            Some(LoadedGridRespawnRecordsLikeCpp::primary_only(
                MapObjectRecord::new_creature(creature).unwrap(),
            ))
        },
    );

    assert_eq!(loader_calls, 1);
    assert_eq!(summary.processed_pool_timers, 1);
    assert_eq!(summary.executed_loaded_grid_respawns, 1);
    assert_eq!(summary.pool_spawn_actions_blocked_loaded_grid, 0);
    assert_eq!(summary.pool_spawn_action_load_plans, Vec::new());
    assert_eq!(summary.blocked_loaded_grid_respawn_add_to_map, 0);
    assert_eq!(
        map.get_respawn_time_like_cpp(SpawnObjectType::Creature, 52601),
        0
    );
    assert!(map.pool_data_like_cpp().is_spawned_creature_like_cpp(52602));
    assert_eq!(
        map.pool_data_like_cpp().get_spawned_objects_like_cpp(526),
        1
    );
    assert!(map.map_object_record(expected_guid).is_some());
    assert_eq!(map.creature_spawn_id_store_count_like_cpp(52602), 1);
}
#[test]
fn process_respawns_pool_loaded_grid_spawn_one_loader_none_keeps_load_plan_evidence_like_cpp() {
    let mut map = test_map();
    let mut store = SpawnStore::new();
    let active = spawn_group(527, SpawnGroupFlags::NONE);
    store.add_object_spawn(
        &spawn_data(SpawnObjectType::GameObject, 52701, active.clone()),
        |_| false,
    );
    store.add_object_spawn(
        &spawn_data(SpawnObjectType::GameObject, 52702, active),
        |_| false,
    );
    map.ensure_grid_loaded(&cell_from_world(0.0, 0.0));
    map.add_respawn_info_like_cpp(respawn_info(SpawnObjectType::GameObject, 52701, 100));
    let mut pool_mgr = PoolMgrLikeCpp::new();
    pool_mgr.insert_template_like_cpp(527, PoolTemplateDataLikeCpp::new(1, 571));
    let mut group = PoolGroupLikeCpp::with_pool_id(PoolMemberKindLikeCpp::GameObject, 527);
    group.add_entry_like_cpp(PoolObjectLikeCpp::new(52701, 0.0), 1);
    group.add_entry_like_cpp(PoolObjectLikeCpp::new(52702, 0.0), 1);
    pool_mgr
        .insert_or_replace_group_like_cpp(PoolMemberKindLikeCpp::GameObject, 527, group)
        .expect("test pool group");
    pool_mgr
        .register_spawn_pool_relation_like_cpp(PoolMemberKindLikeCpp::GameObject, 52701, 527)
        .expect("test spawn pool relation");

    let summary = map.process_due_respawns_composite_safe_side_effects_like_cpp(
        100,
        &store,
        &LinkedRespawnStoreLikeCpp::new(),
        &pool_mgr,
        5,
        false,
        |_, _| false,
        |_, _| 0.0,
        |_candidates, _count| vec![1],
    );

    assert_eq!(summary.processed_pool_timers, 1);
    assert_eq!(summary.executed_loaded_grid_respawns, 0);
    assert_eq!(summary.pool_spawn_actions_blocked_loaded_grid, 1);
    assert_eq!(
        summary.pool_spawn_action_load_plans,
        vec![PoolSpawnActionLoadPlanLikeCpp {
            object_type: SpawnObjectType::GameObject,
            spawn_id: 52702,
            respawn: false,
        }]
    );
    assert_eq!(map.map_object_count(), 0);
    assert_eq!(
        map.get_respawn_time_like_cpp(SpawnObjectType::GameObject, 52701),
        0
    );
    assert!(
        map.pool_data_like_cpp()
            .is_spawned_gameobject_like_cpp(52702)
    );
}
#[test]
fn process_respawns_pool_loaded_grid_add_to_map_failure_counts_and_removes_trigger_timer_like_cpp()
{
    let mut map = test_map();
    let mut store = SpawnStore::new();
    let active = spawn_group(528, SpawnGroupFlags::NONE);
    store.add_object_spawn(
        &spawn_data(SpawnObjectType::Creature, 52801, active.clone()),
        |_| false,
    );
    store.add_object_spawn(
        &spawn_data(SpawnObjectType::Creature, 52802, active),
        |_| false,
    );
    map.ensure_grid_loaded(&cell_from_world(0.0, 0.0));
    map.add_respawn_info_like_cpp(respawn_info(SpawnObjectType::Creature, 52801, 100));
    let mut pool_mgr = PoolMgrLikeCpp::new();
    pool_mgr.insert_template_like_cpp(528, PoolTemplateDataLikeCpp::new(1, 571));
    let mut group = PoolGroupLikeCpp::with_pool_id(PoolMemberKindLikeCpp::Creature, 528);
    group.add_entry_like_cpp(PoolObjectLikeCpp::new(52801, 0.0), 1);
    group.add_entry_like_cpp(PoolObjectLikeCpp::new(52802, 0.0), 1);
    pool_mgr
        .insert_or_replace_group_like_cpp(PoolMemberKindLikeCpp::Creature, 528, group)
        .expect("test pool group");
    pool_mgr
        .register_spawn_pool_relation_like_cpp(PoolMemberKindLikeCpp::Creature, 52801, 528)
        .expect("test spawn pool relation");
    let expected_guid = guid(HighGuid::Creature, 5280201);

    let summary = map.process_due_respawns_composite_loaded_grid_respawns_like_cpp(
        100,
        &store,
        &LinkedRespawnStoreLikeCpp::new(),
        &pool_mgr,
        5,
        false,
        |_, _| false,
        |_, _| 0.0,
        |_candidates, _count| vec![1],
        true,
        |_map, object_type, spawn_id| {
            assert_eq!(object_type, SpawnObjectType::Creature);
            assert_eq!(spawn_id, 52802);
            let mut creature = test_creature_for_spawn(52802, 5280201, true);
            creature
                .unit_mut()
                .world_mut()
                .object_mut()
                .remove_from_world();
            creature
                .unit_mut()
                .world_mut()
                .relocate(Position::xyz(1_000_000.0, 1_000_000.0, 0.0));
            Some(LoadedGridRespawnRecordsLikeCpp::primary_only(
                MapObjectRecord::new_creature(creature).unwrap(),
            ))
        },
    );

    assert_eq!(summary.processed_pool_timers, 1);
    assert_eq!(summary.executed_loaded_grid_respawns, 0);
    assert_eq!(summary.blocked_loaded_grid_respawn_add_to_map, 1);
    assert_eq!(summary.pool_spawn_actions_blocked_loaded_grid, 0);
    assert_eq!(
        map.get_respawn_time_like_cpp(SpawnObjectType::Creature, 52801),
        0
    );
    assert!(map.pool_data_like_cpp().is_spawned_creature_like_cpp(52802));
    assert!(map.map_object_record(expected_guid).is_none());
    assert_eq!(map.creature_spawn_id_store_count_like_cpp(52802), 0);
}
#[test]
fn process_respawns_pool_timer_updates_pool_plan_removes_timer_and_continues_like_cpp() {
    let mut map = test_map();
    let mut store = SpawnStore::new();
    let active = spawn_group(14, SpawnGroupFlags::NONE);
    let inactive = spawn_group(15, SpawnGroupFlags::MANUAL_SPAWN);
    store.add_object_spawn(&spawn_data(SpawnObjectType::GameObject, 45, active), |_| {
        false
    });
    store.add_object_spawn(&spawn_data(SpawnObjectType::Creature, 46, inactive), |_| {
        false
    });
    map.add_respawn_info_like_cpp(respawn_info(SpawnObjectType::GameObject, 45, 90));
    map.add_respawn_info_like_cpp(respawn_info(SpawnObjectType::Creature, 46, 100));
    let mut pool_mgr = PoolMgrLikeCpp::new();
    pool_mgr.insert_template_like_cpp(55, PoolTemplateDataLikeCpp::new(1, 571));
    let mut group = PoolGroupLikeCpp::with_pool_id(PoolMemberKindLikeCpp::GameObject, 55);
    group.add_entry_like_cpp(PoolObjectLikeCpp::new(45, 0.0), 1);
    group.add_entry_like_cpp(PoolObjectLikeCpp::new(145, 0.0), 1);
    pool_mgr
        .insert_or_replace_group_like_cpp(PoolMemberKindLikeCpp::GameObject, 55, group)
        .expect("test pool group");
    pool_mgr
        .register_spawn_pool_relation_like_cpp(PoolMemberKindLikeCpp::GameObject, 45, 55)
        .expect("test spawn pool relation");

    let summary = map.process_due_respawns_composite_safe_side_effects_like_cpp(
        100,
        &store,
        &LinkedRespawnStoreLikeCpp::new(),
        &pool_mgr,
        5,
        false,
        |_, _| false,
        |_, _| 0.0,
        |_candidates, count| (0..count).collect(),
    );

    assert_eq!(summary.processed_pool_timers, 1);
    assert_eq!(summary.processed_unloaded_grid_respawns, 0);
    assert_eq!(summary.pool_update_plans.len(), 1);
    assert_eq!(summary.blocked_pool_plan_errors, Vec::new());
    assert_eq!(summary.blocked_pool_runtime, 0);
    assert_eq!(summary.deleted_inactive_spawn_group, 1);
    assert!(map.pool_data_like_cpp().is_spawned_gameobject_like_cpp(145));
    assert_eq!(map.pool_data_like_cpp().get_spawned_objects_like_cpp(55), 1);
    assert_eq!(
        map.get_respawn_time_like_cpp(SpawnObjectType::GameObject, 45),
        0
    );
    assert_eq!(
        map.get_respawn_time_like_cpp(SpawnObjectType::Creature, 46),
        0
    );
}
#[test]
fn process_respawns_pool_spawn_action_reports_unloaded_loaded_and_missing_spawn_data_like_cpp() {
    let mut map = test_map();
    let mut store = SpawnStore::new();
    let active = spawn_group(33, SpawnGroupFlags::NONE);
    store.add_object_spawn(
        &spawn_data(SpawnObjectType::Creature, 76, active.clone()),
        |_| false,
    );
    let mut unloaded_spawn = spawn_data(SpawnObjectType::GameObject, 77, active);
    unloaded_spawn.spawn_point = crate::spawn::SpawnPosition::new(1_000.0, 1_000.0, 0.0, 0.0);
    store.add_object_spawn(&unloaded_spawn, |_| false);
    let loaded_cell = cell_from_world(0.0, 0.0);
    map.ensure_grid_loaded(&loaded_cell);
    let plan = PoolTypedSpawnPlanLikeCpp {
        kind: PoolMemberKindLikeCpp::Creature,
        pool_id: 176,
        trigger_from: 0,
        max_limit: Some(1),
        object_plan: Some(PoolSpawnObjectPlanLikeCpp {
            actions: vec![
                PoolSpawnObjectActionLikeCpp::SpawnOne {
                    kind: PoolMemberKindLikeCpp::Creature,
                    guid: 76,
                },
                PoolSpawnObjectActionLikeCpp::SpawnOne {
                    kind: PoolMemberKindLikeCpp::GameObject,
                    guid: 77,
                },
                PoolSpawnObjectActionLikeCpp::SpawnOne {
                    kind: PoolMemberKindLikeCpp::Creature,
                    guid: 78,
                },
                PoolSpawnObjectActionLikeCpp::SpawnOne {
                    kind: PoolMemberKindLikeCpp::Pool,
                    guid: 179,
                },
            ],
            selected: vec![],
            despawned_trigger: None,
            respawned_trigger: false,
            ..PoolSpawnObjectPlanLikeCpp::default()
        }),
        skip_reason: None,
    };
    let mut summary = ProcessRespawnsSafeSideEffectsSummaryLikeCpp::default();

    map.apply_pool_typed_spawn_plan_safe_map_actions_like_cpp(&plan, &store, &mut summary);

    assert_eq!(summary.pool_spawn_actions_blocked_loaded_grid, 1);
    assert_eq!(summary.pool_spawn_actions_skipped_unloaded_grid, 1);
    assert_eq!(summary.pool_spawn_actions_missing_spawn_data, 1);
    assert_eq!(summary.pool_unsupported_action_kind, 1);
    assert_eq!(
        summary.pool_spawn_action_load_plans,
        vec![PoolSpawnActionLoadPlanLikeCpp {
            object_type: SpawnObjectType::Creature,
            spawn_id: 76,
            respawn: false,
        }]
    );
}
