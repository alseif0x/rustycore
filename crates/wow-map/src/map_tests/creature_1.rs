//! Creature scenarios for [`super`].
//!
//! Split out of map_tests.rs under #628; assertions and
//! registrations are unchanged and shared fixtures stay in the parent module.

use super::*;

#[test]
fn active_non_player_add_remove_creature_updates_set_and_unload_lock_like_cpp() {
    let mut map = test_map();
    let respawn_position = Position::xyz(1.0, 2.0, 3.0);
    let respawn_cell = Cell::from_world(respawn_position.x, respawn_position.y);
    let respawn_grid = GridCoord::new(respawn_cell.grid_x(), respawn_cell.grid_y());
    map.ensure_grid_loaded(&cell_from_grid_center(respawn_grid));
    let mut creature = test_creature_for_spawn(48502, 4850201, true);
    let guid = creature.guid();
    creature.set_ai_home_position(respawn_position);
    creature.unit_mut().world_mut().set_active(true);
    creature
        .unit_mut()
        .world_mut()
        .object_mut()
        .remove_from_world();

    let add = map
        .add_map_object_record_to_map_like_cpp(MapObjectRecord::new_creature(creature).unwrap())
        .unwrap();
    let add_active = add
        .add_to_map_tail
        .unwrap()
        .add_to_active
        .expect("active exact typed Creature should consume AddToActive seam");

    assert_eq!(
        add_active.status,
        ActiveNonPlayerMutationStatusLikeCpp::Mutated
    );
    assert!(add_active.inserted_in_active_set);
    assert_eq!(
        add_active.unload_lock.unwrap().respawn_grid,
        Some(respawn_grid)
    );
    assert!(map.is_active_non_player_like_cpp(guid));
    assert_eq!(
        map.get_ngrid(respawn_grid)
            .unwrap()
            .info()
            .unload_active_lock_count(),
        1
    );

    let remove = map.remove_from_map_like_cpp(guid, true).unwrap();
    let remove_active = remove.remove_from_active.unwrap();
    assert!(remove_active.removed_from_active_set);
    assert_eq!(
        remove_active.unload_lock.unwrap().respawn_grid,
        Some(respawn_grid)
    );
    assert!(!map.is_active_non_player_like_cpp(guid));
    assert_eq!(
        map.get_ngrid(respawn_grid)
            .unwrap()
            .info()
            .unload_active_lock_count(),
        0
    );
}
#[test]
fn gameobject_add_to_owner_dispatches_creature_ai_summon_boundary_like_cpp() {
    let mut map = test_map();
    let mut owner = test_creature_for_spawn(48207, 4820701, true);
    let owner_guid = owner.guid();
    owner
        .unit_mut()
        .subsystems_mut()
        .ai
        .set_active(Some("NullCreatureAI"));
    let gameobject = test_gameobject_for_spawn(48207, 4820702);
    let guid = gameobject.world().guid();

    map.insert_map_object_record(MapObjectRecord::new_creature(owner).unwrap())
        .unwrap();
    map.insert_map_object_record(MapObjectRecord::new_game_object(gameobject).unwrap())
        .unwrap();

    let add_owner = map.gameobject_add_to_owner_like_cpp(owner_guid, guid);

    assert!(add_owner.registered_owned_gameobject);
    assert!(add_owner.creature_ai_callback_represented);
    let owner = map
        .map_object_record(owner_guid)
        .and_then(MapObjectRecord::creature)
        .unwrap();
    assert_eq!(
        owner.unit().subsystems().ai.just_summoned_gameobject_count,
        1
    );

    let mut disabled_map = test_map();
    let disabled_owner = test_creature_for_spawn(48208, 4820801, true);
    let disabled_owner_guid = disabled_owner.guid();
    let disabled_gameobject = test_gameobject_for_spawn(48208, 4820802);
    let disabled_guid = disabled_gameobject.world().guid();
    disabled_map
        .insert_map_object_record(MapObjectRecord::new_creature(disabled_owner).unwrap())
        .unwrap();
    disabled_map
        .insert_map_object_record(MapObjectRecord::new_game_object(disabled_gameobject).unwrap())
        .unwrap();

    let disabled_add_owner =
        disabled_map.gameobject_add_to_owner_like_cpp(disabled_owner_guid, disabled_guid);
    assert!(disabled_add_owner.registered_owned_gameobject);
    assert!(!disabled_add_owner.creature_ai_callback_represented);
}
#[test]
fn gameobject_remove_from_owner_dispatches_creature_ai_despawn_boundary_like_cpp() {
    let mut map = test_map();
    let mut owner = test_creature_for_spawn(48204, 4820401, true);
    let owner_guid = owner.guid();
    owner
        .unit_mut()
        .subsystems_mut()
        .ai
        .set_active(Some("NullCreatureAI"));

    let mut gameobject = test_gameobject_for_spawn(48204, 4820402);
    let guid = gameobject.world().guid();
    owner
        .unit_mut()
        .subsystems_mut()
        .control
        .register_owned_gameobject_like_cpp(guid);
    gameobject.set_owner_guid_like_cpp(owner_guid);

    map.insert_map_object_record(MapObjectRecord::new_creature(owner).unwrap())
        .unwrap();
    map.insert_map_object_record(MapObjectRecord::new_game_object(gameobject).unwrap())
        .unwrap();

    let remove_owner = map
        .remove_from_map_like_cpp(guid, true)
        .unwrap()
        .gameobject_remove_from_owner
        .unwrap();

    assert!(remove_owner.owner_found_as_unit_like);
    assert!(remove_owner.creature_ai_callback_represented);
    let owner = map
        .map_object_record(owner_guid)
        .and_then(MapObjectRecord::creature)
        .unwrap();
    assert_eq!(
        owner
            .unit()
            .subsystems()
            .ai
            .summoned_gameobject_despawn_count,
        1
    );

    let mut disabled_map = test_map();
    let disabled_owner = test_creature_for_spawn(48205, 4820501, true);
    let disabled_owner_guid = disabled_owner.guid();
    let mut disabled_gameobject = test_gameobject_for_spawn(48205, 4820502);
    let disabled_guid = disabled_gameobject.world().guid();
    disabled_gameobject.set_owner_guid_like_cpp(disabled_owner_guid);
    disabled_map
        .insert_map_object_record(MapObjectRecord::new_creature(disabled_owner).unwrap())
        .unwrap();
    disabled_map
        .insert_map_object_record(MapObjectRecord::new_game_object(disabled_gameobject).unwrap())
        .unwrap();

    let disabled_remove_owner = disabled_map
        .remove_from_map_like_cpp(disabled_guid, true)
        .unwrap()
        .gameobject_remove_from_owner
        .unwrap();
    assert!(disabled_remove_owner.owner_found_as_unit_like);
    assert!(!disabled_remove_owner.creature_ai_callback_represented);
}
#[test]
fn guid_sequence_creature_starts_at_one_like_cpp() {
    let mut map = test_map();

    assert_eq!(map.generate_low_guid_like_cpp(HighGuid::Creature), Ok(1));
    assert_eq!(map.generate_low_guid_like_cpp(HighGuid::Creature), Ok(2));
    assert_eq!(map.get_max_low_guid_like_cpp(HighGuid::Creature), Ok(3));
}
#[test]
fn guid_sequence_creature_and_gameobject_are_independent_like_cpp() {
    let mut map = test_map();

    assert_eq!(map.generate_low_guid_like_cpp(HighGuid::Creature), Ok(1));
    assert_eq!(map.generate_low_guid_like_cpp(HighGuid::GameObject), Ok(1));
    assert_eq!(map.generate_low_guid_like_cpp(HighGuid::Creature), Ok(2));
    assert_eq!(map.get_max_low_guid_like_cpp(HighGuid::GameObject), Ok(2));
}
#[test]
fn guid_sequence_accepts_non_creature_gameobject_map_sources_like_cpp() {
    let mut map = test_map();

    assert_eq!(map.generate_low_guid_like_cpp(HighGuid::AreaTrigger), Ok(1));
    assert_eq!(
        map.generate_low_guid_like_cpp(HighGuid::DynamicObject),
        Ok(1)
    );
    assert_eq!(map.generate_low_guid_like_cpp(HighGuid::AreaTrigger), Ok(2));
    assert_eq!(
        map.get_max_low_guid_like_cpp(HighGuid::DynamicObject),
        Ok(2)
    );
}
#[test]
fn select_creature_level_fixed_path_does_not_consume_rng_like_cpp() {
    let mut fixed_then_variable = test_map();
    fixed_then_variable.seed_creature_level_rng_for_tests_like_cpp(0x407);
    assert_eq!(
        fixed_then_variable.select_creature_level_like_cpp(19, 19),
        19
    );
    let after_fixed = fixed_then_variable.select_creature_level_like_cpp(18, 20);

    let mut variable_only = test_map();
    variable_only.seed_creature_level_rng_for_tests_like_cpp(0x407);
    let without_fixed = variable_only.select_creature_level_like_cpp(18, 20);

    assert_eq!(after_fixed, without_fixed);
    assert!((18..=20).contains(&after_fixed));
}
#[test]
fn game_event_smart_ai_candidates_count_exact_in_world_creature_gameobject_only_like_cpp() {
    let mut map = test_map();

    map.insert_map_object_record(
        MapObjectRecord::new_creature(test_creature_for_spawn(55301, 5530101, true)).unwrap(),
    )
    .unwrap();

    let mut not_in_world_creature = test_creature_for_spawn(55302, 5530102, true);
    not_in_world_creature
        .unit_mut()
        .world_mut()
        .object_mut()
        .remove_from_world();
    map.insert_map_object_record(MapObjectRecord::new_creature(not_in_world_creature).unwrap())
        .unwrap();

    map.insert_map_object_record(
        MapObjectRecord::new_game_object(test_gameobject_for_spawn(55303, 5530103)).unwrap(),
    )
    .unwrap();

    let mut not_in_world_gameobject = test_gameobject_for_spawn(55304, 5530104);
    not_in_world_gameobject
        .world_mut()
        .object_mut()
        .remove_from_world();
    map.insert_map_object_record(
        MapObjectRecord::new_game_object(not_in_world_gameobject).unwrap(),
    )
    .unwrap();

    let generic = world_object_with_counter(HighGuid::GameObject, 5530105, 571, 7, true);
    map.insert_map_object(AccessorObjectKind::GameObject, generic)
        .unwrap();
    map.insert_map_object_record(
        MapObjectRecord::new_transport(test_transport(553_106, true)).unwrap(),
    )
    .unwrap();

    let summary = map.game_event_smart_ai_script_candidates_like_cpp();

    assert_eq!(summary.maps_visited, 1);
    assert_eq!(summary.in_world_creature_candidates, 1);
    assert_eq!(summary.in_world_gameobject_candidates, 1);
    assert_eq!(summary.creature_ai_enabled_unrepresented, 1);
    assert_eq!(summary.script_dispatch_unrepresented, 2);
}
#[test]
fn grid_unload_actions_apply_to_map_owned_creature_record() {
    let mut map = test_map();
    let creature_guid = guid(HighGuid::Creature, 3711);
    let mut creature = test_creature_for_spawn(371, 3711, true);
    creature.unit_mut().world_mut().set_current_cell(3, 4);
    map.insert_map_object_record(MapObjectRecord::new_creature(creature).unwrap())
        .unwrap();

    let outcomes = apply_grid_unload_actions(
        &mut map,
        [
            GridUnloadAction::CreatureRespawnRelocation(creature_guid),
            GridUnloadAction::CleanupsBeforeDelete(GridObjectKind::Creature, creature_guid),
            GridUnloadAction::DeleteObject(GridObjectKind::Creature, creature_guid),
        ],
    );

    assert_eq!(outcomes, vec![GridUnloadApplyOutcome::Applied; 3]);
    assert_eq!(map.map_object_count(), 1);
    let creature = map
        .map_object_record(creature_guid)
        .unwrap()
        .creature()
        .unwrap();
    assert!(creature.grid_unload_respawn_relocation_requested());
    assert_eq!(creature.cleanup_before_delete_count(), 1);
    assert!(creature.grid_unload_delete_requested());
    assert_eq!(creature.unit().world().current_cell(), None);
}
#[test]
fn spawned_pool_data_creature_gameobject_and_dispatcher_like_cpp() {
    let mut pool_data = SpawnedPoolDataLikeCpp::new();

    assert_eq!(pool_data.get_spawned_objects_like_cpp(7), 0);
    assert_eq!(
        pool_data.is_spawned_object_like_cpp(SpawnObjectType::Creature, 101),
        Ok(false)
    );
    assert_eq!(
        pool_data.is_spawned_object_like_cpp(SpawnObjectType::GameObject, 202),
        Ok(false)
    );
    assert_eq!(
        pool_data.is_spawned_object_like_cpp(SpawnObjectType::AreaTrigger, 303),
        Err(SpawnedPoolDataErrorLikeCpp::UnsupportedSpawnObjectType(
            SpawnObjectType::AreaTrigger
        ))
    );

    assert_eq!(
        pool_data.add_spawn_like_cpp(SpawnObjectType::Creature, 101, 7),
        Ok(())
    );
    assert_eq!(
        pool_data.add_spawn_like_cpp(SpawnObjectType::GameObject, 202, 7),
        Ok(())
    );
    assert!(pool_data.is_spawned_creature_like_cpp(101));
    assert!(pool_data.is_spawned_gameobject_like_cpp(202));
    assert_eq!(pool_data.get_spawned_objects_like_cpp(7), 2);
    assert_eq!(
        pool_data.spawned_objects_like_cpp(),
        vec![
            (SpawnObjectType::Creature, 101),
            (SpawnObjectType::GameObject, 202),
        ]
    );
}
#[test]
fn process_respawns_loaded_grid_creature_loader_adds_record_and_removes_timer_like_cpp() {
    let mut map = test_map();
    let mut store = SpawnStore::new();
    let active = spawn_group(397, SpawnGroupFlags::NONE);
    store.add_object_spawn(
        &spawn_data(SpawnObjectType::Creature, 39701, active),
        |_| false,
    );
    map.ensure_grid_loaded(&cell_from_grid_center(GridCoord::new(7, 0)));
    map.add_respawn_info_like_cpp(respawn_info(SpawnObjectType::Creature, 39701, 100));
    let expected_guid = guid(HighGuid::Creature, 3970101);
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
        |map, object_type, spawn_id| {
            loader_calls += 1;
            assert_eq!(object_type, SpawnObjectType::Creature);
            assert_eq!(spawn_id, 39701);
            let low = map
                .generate_low_guid_like_cpp(HighGuid::Creature)
                .expect("map-owned Creature low-guid allocator");
            assert_eq!(low, 1);
            let mut creature = test_creature_for_spawn(39701, 3970101, true);
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
    assert_eq!(summary.executed_loaded_grid_respawns, 1);
    assert_eq!(summary.blocked_loaded_grid_respawn_loads, 0);
    assert_eq!(summary.blocked_loaded_grid_respawn_add_to_map, 0);
    assert_eq!(summary.blocked_do_respawn_runtime, 0);
    assert_eq!(
        map.get_respawn_time_like_cpp(SpawnObjectType::Creature, 39701),
        0
    );
    assert_eq!(map.creature_spawn_id_store_count_like_cpp(39701), 1);
    let record = map.map_object_record(expected_guid).unwrap();
    assert!(record.object().object().is_in_world());
    assert!(record.creature().is_some());
    assert!(map.get_creature_by_spawn_id_like_cpp(39701).is_some());
    let cell = Cell::from_world(record.object().position().x, record.object().position().y);
    let grid = map
        .get_ngrid(GridCoord::new(cell.grid_x(), cell.grid_y()))
        .unwrap();
    let local_cell = grid
        .get_grid_type(cell.cell_x(), cell.cell_y())
        .expect("record inserted into target cell");
    assert!(local_cell.grid_objects.creatures.contains(&expected_guid));
}
#[test]
fn process_respawns_pool_plan_despawn_one_removes_live_creature_and_gameobject_like_cpp() {
    let mut map = test_map();
    let mut store = SpawnStore::new();
    let active = spawn_group(31, SpawnGroupFlags::NONE);
    store.add_object_spawn(
        &spawn_data(SpawnObjectType::Creature, 71, active.clone()),
        |_| false,
    );
    store.add_object_spawn(&spawn_data(SpawnObjectType::GameObject, 72, active), |_| {
        false
    });
    map.insert_map_object_record(
        MapObjectRecord::new_creature(test_creature_for_spawn(71, 7101, true)).unwrap(),
    )
    .unwrap();
    map.insert_map_object_record(
        MapObjectRecord::new_game_object(test_gameobject_for_spawn(72, 7201)).unwrap(),
    )
    .unwrap();
    map.add_respawn_info_like_cpp(respawn_info(SpawnObjectType::Creature, 71, 100));
    map.add_respawn_info_like_cpp(respawn_info(SpawnObjectType::GameObject, 72, 100));

    assert_eq!(
        map.pool_data_mut_like_cpp()
            .add_spawn_like_cpp(SpawnObjectType::Creature, 71, 171),
        Ok(())
    );
    assert_eq!(
        map.pool_data_mut_like_cpp()
            .add_spawn_like_cpp(SpawnObjectType::GameObject, 72, 172),
        Ok(())
    );
    let mut pool_mgr = PoolMgrLikeCpp::new();
    pool_mgr.insert_template_like_cpp(171, PoolTemplateDataLikeCpp::new(0, 571));
    pool_mgr.insert_template_like_cpp(172, PoolTemplateDataLikeCpp::new(0, 571));
    let mut creature_group = PoolGroupLikeCpp::with_pool_id(PoolMemberKindLikeCpp::Creature, 171);
    creature_group.add_entry_like_cpp(PoolObjectLikeCpp::new(71, 0.0), 1);
    pool_mgr
        .insert_or_replace_group_like_cpp(PoolMemberKindLikeCpp::Creature, 171, creature_group)
        .expect("test creature pool group");
    pool_mgr
        .register_spawn_pool_relation_like_cpp(PoolMemberKindLikeCpp::Creature, 71, 171)
        .expect("test creature pool relation");
    let mut gameobject_group =
        PoolGroupLikeCpp::with_pool_id(PoolMemberKindLikeCpp::GameObject, 172);
    gameobject_group.add_entry_like_cpp(PoolObjectLikeCpp::new(72, 0.0), 1);
    pool_mgr
        .insert_or_replace_group_like_cpp(PoolMemberKindLikeCpp::GameObject, 172, gameobject_group)
        .expect("test gameobject pool group");
    pool_mgr
        .register_spawn_pool_relation_like_cpp(PoolMemberKindLikeCpp::GameObject, 72, 172)
        .expect("test gameobject pool relation");

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

    assert_eq!(summary.processed_pool_timers, 2);
    assert_eq!(summary.pool_objects_removed, 2);
    assert_eq!(summary.pool_stale_index_entries, 0);
    assert_eq!(summary.pool_remove_errors, 0);
    assert_eq!(map.map_object_count(), 0);
    assert_eq!(map.creature_spawn_id_store_count_like_cpp(71), 0);
    assert_eq!(map.gameobject_spawn_id_store_count_like_cpp(72), 0);
    assert_eq!(
        map.get_respawn_time_like_cpp(SpawnObjectType::Creature, 71),
        0
    );
    assert_eq!(
        map.get_respawn_time_like_cpp(SpawnObjectType::GameObject, 72),
        0
    );
}
#[test]
fn despawn_pool_facade_removes_live_creature_and_gameobject_from_map_owned_state_like_cpp() {
    let mut map = test_map();
    map.insert_map_object_record(
        MapObjectRecord::new_creature(test_creature_for_spawn(528101, 52810101, true)).unwrap(),
    )
    .unwrap();
    map.insert_map_object_record(
        MapObjectRecord::new_game_object(test_gameobject_for_spawn(528102, 52810201)).unwrap(),
    )
    .unwrap();
    map.pool_data_mut_like_cpp()
        .add_spawn_like_cpp(SpawnObjectType::Creature, 528101, 5281)
        .expect("test creature pool state");
    map.pool_data_mut_like_cpp()
        .add_spawn_like_cpp(SpawnObjectType::GameObject, 528102, 5281)
        .expect("test gameobject pool state");
    let mut pool_mgr = PoolMgrLikeCpp::new();
    let mut creature_group = PoolGroupLikeCpp::with_pool_id(PoolMemberKindLikeCpp::Creature, 5281);
    creature_group.add_entry_like_cpp(PoolObjectLikeCpp::new(528101, 0.0), 1);
    pool_mgr
        .insert_or_replace_group_like_cpp(PoolMemberKindLikeCpp::Creature, 5281, creature_group)
        .expect("test creature pool group");
    let mut gameobject_group =
        PoolGroupLikeCpp::with_pool_id(PoolMemberKindLikeCpp::GameObject, 5281);
    gameobject_group.add_entry_like_cpp(PoolObjectLikeCpp::new(528102, 0.0), 1);
    pool_mgr
        .insert_or_replace_group_like_cpp(PoolMemberKindLikeCpp::GameObject, 5281, gameobject_group)
        .expect("test gameobject pool group");

    let summary = map
        .despawn_pool_safe_map_actions_like_cpp(&pool_mgr, 5281, false)
        .expect("despawn pool plan");

    assert_eq!(summary.pool_objects_removed, 2);
    assert_eq!(summary.pool_respawn_timers_removed, 0);
    assert_eq!(summary.pool_unsupported_action_kind, 0);
    assert_eq!(map.map_object_count(), 0);
    assert_eq!(map.creature_spawn_id_store_count_like_cpp(528101), 0);
    assert_eq!(map.gameobject_spawn_id_store_count_like_cpp(528102), 0);
    assert!(
        !map.pool_data_like_cpp()
            .is_spawned_creature_like_cpp(528101)
    );
    assert!(
        !map.pool_data_like_cpp()
            .is_spawned_gameobject_like_cpp(528102)
    );
    assert_eq!(
        map.pool_data_like_cpp().get_spawned_objects_like_cpp(5281),
        0
    );
}
#[test]
fn despawn_pool_facade_always_delete_removes_non_spawned_creature_gameobject_timers_not_pool_like_cpp()
 {
    let mut map = test_map();
    map.add_respawn_info_like_cpp(respawn_info(SpawnObjectType::Creature, 528201, 200));
    map.add_respawn_info_like_cpp(respawn_info(SpawnObjectType::GameObject, 528202, 200));
    let mut pool_mgr = PoolMgrLikeCpp::new();
    let mut creature_group = PoolGroupLikeCpp::with_pool_id(PoolMemberKindLikeCpp::Creature, 5282);
    creature_group.add_entry_like_cpp(PoolObjectLikeCpp::new(528201, 0.0), 1);
    pool_mgr
        .insert_or_replace_group_like_cpp(PoolMemberKindLikeCpp::Creature, 5282, creature_group)
        .expect("test creature pool group");
    let mut gameobject_group =
        PoolGroupLikeCpp::with_pool_id(PoolMemberKindLikeCpp::GameObject, 5282);
    gameobject_group.add_entry_like_cpp(PoolObjectLikeCpp::new(528202, 0.0), 1);
    pool_mgr
        .insert_or_replace_group_like_cpp(PoolMemberKindLikeCpp::GameObject, 5282, gameobject_group)
        .expect("test gameobject pool group");
    let mut pool_group = PoolGroupLikeCpp::with_pool_id(PoolMemberKindLikeCpp::Pool, 5282);
    pool_group.add_entry_like_cpp(PoolObjectLikeCpp::new(5283, 0.0), 1);
    pool_mgr
        .insert_or_replace_group_like_cpp(PoolMemberKindLikeCpp::Pool, 5282, pool_group)
        .expect("test child-pool relation group");

    let summary = map
        .despawn_pool_safe_map_actions_like_cpp(&pool_mgr, 5282, true)
        .expect("despawn pool plan");

    assert_eq!(summary.pool_respawn_timers_removed, 2);
    assert_eq!(summary.pool_respawn_timers_missing, 0);
    assert_eq!(summary.pool_objects_removed, 0);
    assert_eq!(summary.pool_unsupported_action_kind, 0);
    assert_eq!(
        map.get_respawn_time_like_cpp(SpawnObjectType::Creature, 528201),
        0
    );
    assert_eq!(
        map.get_respawn_time_like_cpp(SpawnObjectType::GameObject, 528202),
        0
    );
}
#[test]
fn check_respawn_live_object_guard_alive_creature_same_spawn_clears_timer_like_cpp() {
    let mut map = test_map();
    let mut store = SpawnStore::new();
    let group = spawn_group(21, SpawnGroupFlags::NONE);
    store.add_object_spawn(&spawn_data(SpawnObjectType::Creature, 51, group), |_| false);
    map.insert_map_object_record(
        MapObjectRecord::new_creature(test_creature_for_spawn(51, 51, true)).unwrap(),
    )
    .unwrap();
    let mut info = respawn_info(SpawnObjectType::Creature, 51, 100);

    let outcome =
        map.check_respawn_live_object_guard_like_cpp(&mut info, &store, false, |_, _| false);

    assert_eq!(
        outcome,
        CheckRespawnLiveObjectGuardOutcomeLikeCpp::AliveCreatureBlocksRespawn
    );
    assert_eq!(info.respawn_time, 0);
}
#[test]
fn check_respawn_live_object_guard_dead_creature_same_spawn_allows_like_cpp() {
    let mut map = test_map();
    let mut store = SpawnStore::new();
    let group = spawn_group(22, SpawnGroupFlags::NONE);
    store.add_object_spawn(&spawn_data(SpawnObjectType::Creature, 52, group), |_| false);
    map.insert_map_object_record(
        MapObjectRecord::new_creature(test_creature_for_spawn(52, 52, false)).unwrap(),
    )
    .unwrap();
    let mut info = respawn_info(SpawnObjectType::Creature, 52, 100);

    let outcome =
        map.check_respawn_live_object_guard_like_cpp(&mut info, &store, false, |_, _| false);

    assert_eq!(outcome, CheckRespawnLiveObjectGuardOutcomeLikeCpp::Allowed);
    assert_eq!(info.respawn_time, 100);
}
#[test]
fn spawn_id_store_two_live_creatures_same_spawn_blocks_respawn_like_cpp() {
    let mut map = test_map();
    let mut store = SpawnStore::new();
    let group = spawn_group(26, SpawnGroupFlags::NONE);
    store.add_object_spawn(&spawn_data(SpawnObjectType::Creature, 57, group), |_| false);

    map.insert_map_object_record(
        MapObjectRecord::new_creature(test_creature_for_spawn(57, 5701, true)).unwrap(),
    )
    .unwrap();
    map.insert_map_object_record(
        MapObjectRecord::new_creature(test_creature_for_spawn(57, 5702, true)).unwrap(),
    )
    .unwrap();

    assert_eq!(map.creature_spawn_id_store_count_like_cpp(57), 2);
    let mut info = respawn_info(SpawnObjectType::Creature, 57, 100);
    let outcome =
        map.check_respawn_live_object_guard_like_cpp(&mut info, &store, false, |_, _| false);

    assert_eq!(
        outcome,
        CheckRespawnLiveObjectGuardOutcomeLikeCpp::AliveCreatureBlocksRespawn
    );
    assert_eq!(info.respawn_time, 0);
}
#[test]
fn game_event_change_equip_or_model_two_live_creatures_same_spawn_mutates_equipment_like_cpp() {
    let mut map = test_map();
    map.insert_map_object_record(
        MapObjectRecord::new_creature(test_creature_for_spawn(157, 15701, true)).unwrap(),
    )
    .unwrap();
    map.insert_map_object_record(
        MapObjectRecord::new_creature(test_creature_for_spawn(157, 15702, true)).unwrap(),
    )
    .unwrap();

    let outcome = map.change_game_event_equip_or_model_by_spawn_id_like_cpp(157, 9, 0, false);

    assert_eq!(outcome.indexed_guids, 2);
    assert_eq!(outcome.live_creatures_mutated, 2);
    assert_eq!(outcome.equipment_changed, 2);
    for guid in map.creature_spawn_id_store_guids_like_cpp(157) {
        let creature = map
            .map_object_record(guid)
            .and_then(MapObjectRecord::creature)
            .unwrap();
        assert_eq!(creature.equipment_id(), 9);
    }
}
#[test]
fn spawn_id_store_removing_creatures_prunes_index_and_guard_allows_like_cpp() {
    let mut map = test_map();
    let mut store = SpawnStore::new();
    let group = spawn_group(27, SpawnGroupFlags::NONE);
    store.add_object_spawn(&spawn_data(SpawnObjectType::Creature, 58, group), |_| false);
    let first_guid = guid(HighGuid::Creature, 5801);
    let second_guid = guid(HighGuid::Creature, 5802);

    map.insert_map_object_record(
        MapObjectRecord::new_creature(test_creature_for_spawn(58, 5801, true)).unwrap(),
    )
    .unwrap();
    map.insert_map_object_record(
        MapObjectRecord::new_creature(test_creature_for_spawn(58, 5802, true)).unwrap(),
    )
    .unwrap();

    assert_eq!(map.creature_spawn_id_store_count_like_cpp(58), 2);
    assert!(map.remove_map_object(first_guid).is_some());
    assert_eq!(map.creature_spawn_id_store_count_like_cpp(58), 1);

    let mut blocked_info = respawn_info(SpawnObjectType::Creature, 58, 100);
    let blocked =
        map.check_respawn_live_object_guard_like_cpp(&mut blocked_info, &store, false, |_, _| {
            false
        });
    assert_eq!(
        blocked,
        CheckRespawnLiveObjectGuardOutcomeLikeCpp::AliveCreatureBlocksRespawn
    );
    assert_eq!(blocked_info.respawn_time, 0);

    assert!(map.remove_map_object(second_guid).is_some());
    assert_eq!(map.creature_spawn_id_store_count_like_cpp(58), 0);

    let mut allowed_info = respawn_info(SpawnObjectType::Creature, 58, 100);
    let allowed =
        map.check_respawn_live_object_guard_like_cpp(&mut allowed_info, &store, false, |_, _| {
            false
        });
    assert_eq!(allowed, CheckRespawnLiveObjectGuardOutcomeLikeCpp::Allowed);
    assert_eq!(allowed_info.respawn_time, 100);
}
#[test]
fn spawn_id_store_replacing_same_guid_moves_creature_spawn_id_like_cpp() {
    let mut map = test_map();
    let guid = guid(HighGuid::Creature, 5901);

    map.insert_map_object_record(
        MapObjectRecord::new_creature(test_creature_for_spawn(59, 5901, true)).unwrap(),
    )
    .unwrap();
    let previous = map
        .insert_map_object_record(
            MapObjectRecord::new_creature(test_creature_for_spawn(60, 5901, true)).unwrap(),
        )
        .unwrap();

    assert!(previous.is_some());
    assert_eq!(map.creature_spawn_id_store_count_like_cpp(59), 0);
    assert_eq!(map.creature_spawn_id_store_count_like_cpp(60), 1);
    assert_eq!(map.creature_spawn_id_store_guids_like_cpp(60), vec![guid]);
}
#[test]
fn world_object_by_spawn_id_creature_prefers_alive_then_fallback_like_cpp() {
    let mut map = test_map();
    let dead_guid = guid(HighGuid::Creature, 7601);
    let alive_guid = guid(HighGuid::Creature, 7602);

    map.insert_map_object_record(
        MapObjectRecord::new_creature(test_creature_for_spawn(76, 7601, false)).unwrap(),
    )
    .unwrap();
    map.insert_map_object_record(
        MapObjectRecord::new_creature(test_creature_for_spawn(76, 7602, true)).unwrap(),
    )
    .unwrap();

    assert_eq!(
        map.creature_spawn_id_store_guids_like_cpp(76),
        vec![dead_guid, alive_guid]
    );
    assert_eq!(
        map.get_creature_by_spawn_id_like_cpp(76)
            .unwrap()
            .unit()
            .world()
            .guid(),
        alive_guid
    );
    assert_eq!(
        map.get_world_object_by_spawn_id_like_cpp(SpawnObjectType::Creature, 76)
            .unwrap()
            .guid(),
        alive_guid
    );

    assert!(map.remove_map_object(alive_guid).is_some());
    assert_eq!(
        map.get_creature_by_spawn_id_like_cpp(76)
            .unwrap()
            .unit()
            .world()
            .guid(),
        dead_guid
    );
}
#[test]
fn spawn_id_store_dead_creature_indexed_but_does_not_block_like_cpp() {
    let mut map = test_map();
    let mut store = SpawnStore::new();
    let group = spawn_group(29, SpawnGroupFlags::NONE);
    store.add_object_spawn(&spawn_data(SpawnObjectType::Creature, 62, group), |_| false);

    map.insert_map_object_record(
        MapObjectRecord::new_creature(test_creature_for_spawn(62, 6201, false)).unwrap(),
    )
    .unwrap();

    assert_eq!(map.creature_spawn_id_store_count_like_cpp(62), 1);
    let mut info = respawn_info(SpawnObjectType::Creature, 62, 100);
    let outcome =
        map.check_respawn_live_object_guard_like_cpp(&mut info, &store, false, |_, _| false);

    assert_eq!(outcome, CheckRespawnLiveObjectGuardOutcomeLikeCpp::Allowed);
    assert_eq!(info.respawn_time, 100);
}
#[test]
fn spawn_group_spawn_loaded_grid_creature_and_gameobject_are_planned_but_not_created_like_cpp() {
    let group = spawn_group(3941, SpawnGroupFlags::NONE);
    let (group, store) = spawn_group_store(
        group,
        vec![
            spawn_data(
                SpawnObjectType::Creature,
                101,
                SpawnGroupTemplateData::default_group(),
            ),
            spawn_data(
                SpawnObjectType::GameObject,
                201,
                SpawnGroupTemplateData::default_group(),
            ),
        ],
    );
    let mut map = test_map();
    map.set_spawn_group_inactive_like_cpp(Some(&group));
    map.load_grid(0.0, 0.0);

    let outcome = map.spawn_group_spawn_like_cpp(Some(&group), false, false, &store);

    assert_eq!(outcome.metadata_entries, 2);
    assert_eq!(
        outcome.applied_active_change,
        Some(SpawnGroupActiveChange::ClearedToggle)
    );
    assert_eq!(outcome.blocked_loaded_grid_creature_loads, 1);
    assert_eq!(outcome.blocked_loaded_grid_gameobject_loads, 1);
    assert_eq!(outcome.blocked_loaded_grid_spawn_loads, 2);
    assert_eq!(outcome.executed_loaded_grid_spawns, 0);
    assert_eq!(outcome.blocked_loaded_grid_spawn_add_to_map, 0);
    assert_eq!(
        outcome.load_plans,
        vec![
            SpawnGroupSpawnLoadPlanLikeCpp {
                object_type: SpawnObjectType::Creature,
                spawn_id: 101,
                force: false,
            },
            SpawnGroupSpawnLoadPlanLikeCpp {
                object_type: SpawnObjectType::GameObject,
                spawn_id: 201,
                force: false,
            },
        ]
    );
    assert_eq!(map.map_object_count(), 0);
    assert!(map.is_spawn_group_active_like_cpp(Some(&group)));
}
