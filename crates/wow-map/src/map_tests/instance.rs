//! Instance scenarios for [`super`].
//!
//! Split out of map_tests.rs under #628; assertions and
//! registrations are unchanged and shared fixtures stay in the parent module.

use super::*;

#[test]
fn active_non_player_active_objects_near_grid_uses_real_active_set_like_cpp() {
    let mut map = test_map();
    let mut gameobject = test_gameobject_for_spawn(48504, 4850401);
    let guid = gameobject.world().guid();
    gameobject.world_mut().set_active(true);
    gameobject.world_mut().object_mut().remove_from_world();
    let object_cell = Cell::from_world(
        gameobject.world().position().x,
        gameobject.world().position().y,
    );
    let object_grid = GridCoord::new(object_cell.grid_x(), object_cell.grid_y());
    map.add_map_object_record_to_map_like_cpp(
        MapObjectRecord::new_game_object(gameobject).unwrap(),
    )
    .unwrap();
    map.unmark_active_cell(object_cell.cell_coord());
    let grid = NGrid::from_coords(
        object_grid.x_coord as i32,
        object_grid.y_coord as i32,
        1000,
        true,
    );

    assert!(map.active_objects_near_grid(&grid));
    let remove = map.remove_from_map_like_cpp(guid, true).unwrap();
    assert!(remove.remove_from_active.unwrap().removed_from_active_set);
    assert!(!map.active_objects_near_grid(&grid));

    let mut stale_map = test_map();
    let mut stale_gameobject = test_gameobject_for_spawn(48504, 4850402);
    stale_gameobject.world_mut().set_active(true);
    stale_gameobject
        .world_mut()
        .object_mut()
        .remove_from_world();
    let stale_guid = stale_gameobject.world().guid();
    stale_map
        .add_map_object_record_to_map_like_cpp(
            MapObjectRecord::new_game_object(stale_gameobject).unwrap(),
        )
        .unwrap();
    stale_map.unmark_active_cell(object_cell.cell_coord());
    stale_map.active_non_players_like_cpp.remove(&stale_guid);
    assert!(!stale_map.active_objects_near_grid(&grid));
}
#[test]
fn map_grid_state_delayed_helper_active_expired_moves_to_idle_and_stops_like_cpp() {
    let mut map = test_map();
    let position = Position::xyz(3_000.0, 3_000.0, 0.0);
    assert!(map.load_grid(position.x, position.y));
    let cell = Cell::from_world(position.x, position.y);
    let coord = GridCoord::new(cell.grid_x(), cell.grid_y());
    let grid = map.get_ngrid_mut(coord).unwrap();
    grid.set_state(GridStateKind::Active);
    grid.info_mut().reset_time_tracker(1);

    let summary = map.update_loaded_grid_states_like_cpp(1);

    assert_eq!(summary.diff_ms, 1);
    assert_eq!(summary.visited, 1);
    assert_eq!(summary.updated, 1);
    assert_eq!(summary.active_to_idle, 1);
    assert_eq!(summary.unloaded, 0);
    assert_eq!(summary.missing_after_snapshot, 0);
    assert_eq!(map.get_ngrid(coord).unwrap().state(), GridStateKind::Idle);
    assert_eq!(map.lifecycle().stops, 1);
}
#[test]
fn guid_sequence_is_map_instance_local_like_cpp() {
    let mut first_map = test_map();
    let mut second_map = test_map();

    assert_eq!(
        first_map.generate_low_guid_like_cpp(HighGuid::Creature),
        Ok(1)
    );
    assert_eq!(
        first_map.generate_low_guid_like_cpp(HighGuid::Creature),
        Ok(2)
    );
    assert_eq!(
        second_map.generate_low_guid_like_cpp(HighGuid::Creature),
        Ok(1)
    );
    assert_eq!(
        second_map.get_max_low_guid_like_cpp(HighGuid::Creature),
        Ok(2)
    );
}
#[test]
fn guid_sequence_rejects_non_map_local_high_guid_like_cpp() {
    let mut map = test_map();

    assert_eq!(
        map.generate_low_guid_like_cpp(HighGuid::Player),
        Err(MapGuidSequenceErrorLikeCpp::UnsupportedSequenceSource {
            high: HighGuid::Player,
        })
    );
}
#[test]
fn map_init_pools_for_map_mutates_map_owned_pool_data_like_cpp() {
    let mut pool_mgr = PoolMgrLikeCpp::new();
    pool_mgr.insert_template_like_cpp(10, PoolTemplateDataLikeCpp::new(1, 571));
    let mut group = PoolGroupLikeCpp::with_pool_id(PoolMemberKindLikeCpp::Creature, 10);
    group.add_entry_like_cpp(PoolObjectLikeCpp::new(101, 0.0), 1);
    pool_mgr
        .insert_or_replace_group_like_cpp(PoolMemberKindLikeCpp::Creature, 10, group)
        .expect("test pool group");
    pool_mgr.add_auto_spawn_pool_like_cpp(571, 10);
    let mut map = test_map();

    let plan = map.init_pools_for_map_like_cpp(
        &pool_mgr,
        |_, _| 0.0,
        |_candidates, count| (0..count).collect(),
    );

    assert_eq!(plan.map_id, 571);
    assert_eq!(plan.planned(), 1);
    assert!(map.pool_data_like_cpp().is_spawned_creature_like_cpp(101));
    assert_eq!(map.pool_data_like_cpp().get_spawned_objects_like_cpp(10), 1);
}
#[test]
fn world_object_los_delegates_to_map_environment_hook_like_cpp() {
    let map = world_object_environment_test_map(
        RecordingWorldObjectTerrain::new(false, INVALID_HEIGHT, INVALID_HEIGHT),
        100.0,
    );
    let mut source = world_object(HighGuid::DynamicObject, 571, 7, true);
    source.relocate(Position::new(1.0, 2.0, 3.0, 0.25));
    let mut target = world_object_with_counter(HighGuid::GameObject, 2, 571, 7, true);
    target.relocate(Position::new(4.0, 5.0, 6.0, 0.75));

    let result = source.is_within_los_in_map(
        &target,
        &map,
        wow_entities::LineOfSightOptions {
            check_dynamic: true,
        },
    );

    assert!(!result);
    assert_eq!(
        map.terrain().los_calls.borrow().as_slice(),
        &[LosCall {
            source_guid: source.guid(),
            target_guid: Some(target.guid()),
            from: Position::new(1.0, 2.0, 3.0, 0.0),
            to: Position::new(4.0, 5.0, 6.0, 0.0),
            check_dynamic: true,
        }]
    );
}
#[test]
fn world_object_map_height_and_floor_delegate_to_map_environment_hook_like_cpp() {
    let map = world_object_environment_test_map(
        RecordingWorldObjectTerrain::new(true, 88.0, 25.0),
        100.0,
    );
    let mut object = world_object(HighGuid::DynamicObject, 571, 7, true);
    object.relocate(Position::new(1.0, 2.0, 3.0, 0.25));
    object.set_static_floor_z(20.0);
    let height_query = WorldObjectHeightQuery {
        vmap: false,
        distance_to_search: 9.0,
    };

    let height = object.get_map_height(&map, 4.0, 5.0, 6.0, height_query);
    let floor = object.get_floor_z(&map);

    assert_eq!(height, 88.0);
    assert_eq!(floor, 25.0);
    assert_eq!(
        map.terrain().height_calls.borrow().as_slice(),
        &[HeightCall {
            object_guid: object.guid(),
            x: 4.0,
            y: 5.0,
            z: 6.5,
            query: height_query,
        }]
    );
    assert_eq!(
        map.terrain().floor_calls.borrow().as_slice(),
        &[FloorCall {
            object_guid: object.guid(),
            position: Position::new(1.0, 2.0, 3.5, 0.25),
            max_search_dist: 50.0,
        }]
    );
}
#[test]
fn map_owned_respawn_get_time_zero_area_trigger_and_inserted_timers_like_cpp() {
    let mut map = test_map();

    assert_eq!(
        map.get_respawn_time_like_cpp(SpawnObjectType::Creature, 10),
        0
    );
    assert_eq!(
        map.get_respawn_time_like_cpp(SpawnObjectType::AreaTrigger, 10),
        0
    );
    assert_eq!(
        map.add_respawn_info_like_cpp(respawn_info(SpawnObjectType::AreaTrigger, 10, 100)),
        AddRespawnInfoOutcomeLikeCpp::RejectedUnsupportedType
    );
    assert_eq!(
        map.add_respawn_info_like_cpp(respawn_info(SpawnObjectType::Creature, 10, 100)),
        AddRespawnInfoOutcomeLikeCpp::Inserted
    );
    assert_eq!(
        map.add_respawn_info_like_cpp(respawn_info(SpawnObjectType::GameObject, 20, 200)),
        AddRespawnInfoOutcomeLikeCpp::Inserted
    );

    assert_eq!(
        map.get_respawn_time_like_cpp(SpawnObjectType::Creature, 10),
        100
    );
    assert_eq!(
        map.get_respawn_time_like_cpp(SpawnObjectType::GameObject, 20),
        200
    );
    assert_eq!(
        map.get_respawn_time_like_cpp(SpawnObjectType::AreaTrigger, 10),
        0
    );
}
#[test]
fn map_owned_respawn_process_due_respawns_delegates_to_owned_store_like_cpp() {
    let mut map = test_map();
    map.add_respawn_info_like_cpp(respawn_info(SpawnObjectType::Creature, 10, 100));
    map.add_respawn_info_like_cpp(respawn_info(SpawnObjectType::GameObject, 20, 200));

    let actions = map.process_due_respawns_like_cpp(
        100,
        |_, _| None,
        |_| CheckRespawnOutcomeLikeCpp::Allowed,
    );

    assert_eq!(
        actions,
        vec![ProcessRespawnActionLikeCpp::DoRespawn {
            object_type: SpawnObjectType::Creature,
            spawn_id: 10,
            grid_id: 7,
        }]
    );
    assert_eq!(
        map.get_respawn_time_like_cpp(SpawnObjectType::Creature, 10),
        0
    );
    assert_eq!(
        map.get_respawn_time_like_cpp(SpawnObjectType::GameObject, 20),
        200
    );

    let future_actions = map.process_due_respawns_like_cpp(
        150,
        |_, _| None,
        |_| CheckRespawnOutcomeLikeCpp::Allowed,
    );
    assert!(future_actions.is_empty());
    assert_eq!(
        map.get_respawn_time_like_cpp(SpawnObjectType::GameObject, 20),
        200
    );
}
#[test]
fn area_trigger_spawn_id_store_multiple_same_spawn_keeps_multimap_cardinality_like_cpp() {
    let mut map = test_map();
    let first_guid = guid(HighGuid::AreaTrigger, 7401);
    let second_guid = guid(HighGuid::AreaTrigger, 7402);

    map.insert_map_object_record(
        MapObjectRecord::new_area_trigger(test_area_trigger_for_spawn(74, 7402)).unwrap(),
    )
    .unwrap();
    map.insert_map_object_record(
        MapObjectRecord::new_area_trigger(test_area_trigger_for_spawn(74, 7401)).unwrap(),
    )
    .unwrap();

    assert_eq!(map.area_trigger_spawn_id_store_count_like_cpp(74), 2);
    assert_eq!(
        map.area_trigger_spawn_id_store_guids_like_cpp(74),
        vec![first_guid, second_guid]
    );
    assert_eq!(
        map.get_area_trigger_by_spawn_id_like_cpp(74)
            .unwrap()
            .world()
            .guid(),
        first_guid
    );
}
#[test]
fn map_constructor_starts_with_empty_grid_slots_like_cpp_pointer_array() {
    let map = test_map();

    assert_eq!(map.map_id(), 571);
    assert_eq!(map.instance_id(), 7);
    assert_eq!(map.spawn_mode(), 1);
    assert_eq!(map.grid_expiry_ms(), 1000);
    assert!(map.grid_unload());
    assert_eq!(map.visibility_range(), 100.0);
    assert_eq!(map.grids.len(), GRID_SLOT_COUNT);
    assert!(map.grids.iter().all(Option::is_none));
}
#[test]
fn map_object_store_inserts_finds_typed_objects_and_removes_by_guid() {
    let mut map = test_map();
    let creature = world_object(HighGuid::Creature, 571, 7, true);
    let gameobject = world_object(HighGuid::GameObject, 571, 7, true);
    let creature_guid = creature.guid();
    let gameobject_guid = gameobject.guid();

    assert!(
        map.insert_map_object(AccessorObjectKind::Creature, creature)
            .unwrap()
            .is_none()
    );
    assert!(
        map.insert_map_object(AccessorObjectKind::GameObject, gameobject)
            .unwrap()
            .is_none()
    );

    assert_eq!(map.map_object_count(), 2);
    assert_eq!(
        map.get_creature(creature_guid).unwrap().guid(),
        creature_guid
    );
    assert_eq!(
        map.get_game_object(gameobject_guid).unwrap().guid(),
        gameobject_guid
    );
    assert!(map.get_game_object(creature_guid).is_none());

    assert_eq!(
        map.remove_map_object(creature_guid)
            .unwrap()
            .object()
            .guid(),
        creature_guid
    );
    assert!(map.get_creature(creature_guid).is_none());
    assert_eq!(map.map_object_count(), 1);
}
#[test]
fn map_object_store_can_hold_typed_player_entity_like_cpp() {
    let mut map = test_map();
    let mut player = Player::new(Some(7), false);
    let player_guid = guid(HighGuid::Player, 42);
    let victim_guid = guid(HighGuid::Creature, 77);
    player
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(player_guid);
    player.unit_mut().world_mut().set_map(571, 7).unwrap();
    player
        .unit_mut()
        .world_mut()
        .relocate(Position::xyz(10.0, 20.0, 30.0));
    player.unit_mut().world_mut().object_mut().add_to_world();
    player.unit_mut().set_attacking(Some(victim_guid));

    map.insert_map_object_record(MapObjectRecord::new_player(player).unwrap())
        .unwrap();

    assert_eq!(map.map_object(player_guid).unwrap().guid(), player_guid);
    assert_eq!(
        map.get_typed_player(player_guid)
            .unwrap()
            .unit()
            .attacking(),
        Some(victim_guid)
    );
    map.get_typed_player_mut(player_guid)
        .unwrap()
        .unit_mut()
        .set_attacking(None);
    assert_eq!(
        map.get_typed_player(player_guid)
            .unwrap()
            .unit()
            .attacking(),
        None
    );
}
#[test]
fn map_object_store_rejects_records_from_other_map_or_instance() {
    let mut map = test_map();
    let other_map_creature = world_object(HighGuid::Creature, 530, 7, true);
    let other_instance_creature = world_object(HighGuid::Creature, 571, 8, true);

    assert!(matches!(
        map.insert_map_object(AccessorObjectKind::Creature, other_map_creature),
        Err(MapObjectStoreError::WrongMap {
            expected_map_id: 571,
            expected_instance_id: 7,
            actual_map_id: 530,
            actual_instance_id: 7,
            ..
        })
    ));
    assert!(matches!(
        map.insert_map_object(AccessorObjectKind::Creature, other_instance_creature),
        Err(MapObjectStoreError::WrongMap {
            expected_map_id: 571,
            expected_instance_id: 7,
            actual_map_id: 571,
            actual_instance_id: 8,
            ..
        })
    ));
    assert_eq!(map.map_object_count(), 0);
}
#[test]
fn add_to_map_like_cpp_creates_grid_marks_world_and_stores_grid_object() {
    let mut map = test_map();
    let creature = world_object(HighGuid::Creature, 571, 7, false);
    let guid = creature.guid();

    let outcome = map
        .add_to_map_like_cpp(AccessorObjectKind::Creature, creature)
        .unwrap();

    assert_eq!(outcome.guid, guid);
    assert!(outcome.inserted);
    assert!(!outcome.already_in_world);
    assert!(outcome.grid_created);
    assert!(!outcome.grid_loaded);
    assert!(outcome.inserted_into_cell);

    let stored = map.get_creature(guid).unwrap();
    assert!(stored.object().is_in_world());
    assert!(stored.object().is_in_grid());
    assert!(!stored.object().is_new_object());
    assert_eq!(
        stored.current_cell(),
        Some((
            outcome.cell.x_coord % MAX_NUMBER_OF_CELLS,
            outcome.cell.y_coord % MAX_NUMBER_OF_CELLS
        ))
    );

    let grid = map.get_ngrid(outcome.grid).unwrap();
    let cell = grid
        .get_grid_type(
            outcome.cell.x_coord % MAX_NUMBER_OF_CELLS,
            outcome.cell.y_coord % MAX_NUMBER_OF_CELLS,
        )
        .unwrap();
    assert!(cell.grid_objects.creatures.contains(&guid));
    assert!(!cell.world_objects.creatures.contains(&guid));
}
#[test]
fn add_to_map_like_cpp_player_is_active_even_without_runtime_active_flag() {
    let mut map = test_map();
    let player = world_object(HighGuid::Player, 571, 7, false);
    let guid = player.guid();

    let outcome = map
        .add_to_map_like_cpp(AccessorObjectKind::Player, player)
        .unwrap();

    assert_eq!(outcome.guid, guid);
    assert!(outcome.grid_loaded);
    assert!(!outcome.grid_created);
    assert!(map.is_grid_loaded(outcome.grid));
    let grid = map.get_ngrid(outcome.grid).unwrap();
    let cell = grid
        .get_grid_type(
            outcome.cell.x_coord % MAX_NUMBER_OF_CELLS,
            outcome.cell.y_coord % MAX_NUMBER_OF_CELLS,
        )
        .unwrap();
    assert!(cell.world_objects.players.contains(&guid));
}
#[test]
fn add_to_map_like_cpp_rejects_invalid_coordinates_before_grid_mutation() {
    let mut map = test_map();
    let mut creature = world_object(HighGuid::Creature, 571, 7, false);
    let guid = creature.guid();
    creature.relocate(Position::xyz(f32::NAN, 0.0, 0.0));

    assert!(matches!(
        map.add_to_map_like_cpp(AccessorObjectKind::Creature, creature),
        Err(AddToMapError::InvalidCoordinates { guid: actual, .. }) if actual == guid
    ));
    assert_eq!(map.map_object_count(), 0);
    assert!(map.terrain().loads.is_empty());
}
#[test]
fn add_to_map_like_cpp_rejects_wrong_map_before_grid_mutation() {
    let mut map = test_map();
    let creature = world_object(HighGuid::Creature, 530, 7, false);

    assert!(matches!(
        map.add_to_map_like_cpp(AccessorObjectKind::Creature, creature),
        Err(AddToMapError::Store(MapObjectStoreError::WrongMap {
            expected_map_id: 571,
            actual_map_id: 530,
            ..
        }))
    ));
    assert_eq!(map.map_object_count(), 0);
    assert!(map.terrain().loads.is_empty());
}
#[test]
fn insert_map_object_record_preserves_shared_typed_authority_for_same_guid_refresh() {
    let player = ObjectGuid::create_player(1, 484_105);

    let mut creature_map = test_map();
    let mut creature = test_creature_for_spawn(484_107, 4_841_050, true);
    let creature_guid = creature.guid();
    assert!(
        creature
            .initialize_shared_loot_authority_like_cpp(money_loot_for_player_like_cpp(
                creature_guid,
                41,
                player,
            ))
            .installed()
    );
    let creature_record = MapObjectRecord::new_creature(creature).unwrap();
    let creature_refresh = creature_record.clone();
    let creature_authority = creature_record
        .creature()
        .unwrap()
        .loot_authority_like_cpp()
        .clone();
    creature_map
        .insert_map_object_record(creature_record)
        .unwrap();
    let creature_lease = poll_immediately_ready(creature_authority.reserve_money_like_cpp(player))
        .expect("the shared Creature authority must grant the uncontended lease");

    let displaced_creature = creature_map
        .insert_map_object_record(creature_refresh)
        .unwrap()
        .expect("same-GUID refresh must return the prior Creature record");

    assert!(
        displaced_creature
            .creature()
            .unwrap()
            .loot_authority_like_cpp()
            .shares_storage_like_cpp(&creature_authority)
    );
    assert_eq!(
        creature_authority.lifecycle_like_cpp(),
        OwnedLootAuthorityLifecycle::Active
    );
    assert_eq!(creature_lease.commit_like_cpp(), Ok(true));
    assert_eq!(
        creature_map
            .map_object_record(creature_guid)
            .and_then(MapObjectRecord::creature)
            .unwrap()
            .loot_authority_like_cpp()
            .shared_snapshot_like_cpp()
            .unwrap()
            .loot
            .coins,
        0
    );

    let mut gameobject_map = test_map();
    let mut gameobject = test_gameobject_for_spawn(484_108, 4_841_060);
    let gameobject_guid = gameobject.world().guid();
    assert!(
        gameobject
            .initialize_shared_loot_authority_like_cpp(money_loot_for_player_like_cpp(
                gameobject_guid,
                43,
                player,
            ))
            .installed()
    );
    let gameobject_record = MapObjectRecord::new_game_object(gameobject).unwrap();
    let gameobject_refresh = gameobject_record.clone();
    let gameobject_authority = gameobject_record
        .game_object()
        .unwrap()
        .loot_authority_like_cpp()
        .clone();
    gameobject_map
        .insert_map_object_record(gameobject_record)
        .unwrap();
    let gameobject_lease =
        poll_immediately_ready(gameobject_authority.reserve_money_like_cpp(player))
            .expect("the shared GameObject authority must grant the uncontended lease");

    let displaced_gameobject = gameobject_map
        .insert_map_object_record(gameobject_refresh)
        .unwrap()
        .expect("same-GUID refresh must return the prior GameObject record");

    assert!(
        displaced_gameobject
            .game_object()
            .unwrap()
            .loot_authority_like_cpp()
            .shares_storage_like_cpp(&gameobject_authority)
    );
    assert_eq!(
        gameobject_authority.lifecycle_like_cpp(),
        OwnedLootAuthorityLifecycle::Active
    );
    assert_eq!(gameobject_lease.commit_like_cpp(), Ok(true));
    assert_eq!(
        gameobject_map
            .map_object_record(gameobject_guid)
            .and_then(MapObjectRecord::game_object)
            .unwrap()
            .loot_authority_like_cpp()
            .shared_snapshot_like_cpp()
            .unwrap()
            .loot
            .coins,
        0
    );
}
#[test]
fn relocate_map_object_like_cpp_same_cell_only_updates_position() {
    let mut map = test_map();
    let creature = world_object(HighGuid::Creature, 571, 7, false);
    let guid = creature.guid();
    let added = map
        .add_to_map_like_cpp(AccessorObjectKind::Creature, creature)
        .unwrap();

    let outcome = map
        .relocate_map_object_like_cpp(guid, Position::xyz(2.0, 3.0, 4.0))
        .unwrap();

    assert!(outcome.relocated);
    assert!(!outcome.moved_between_cells);
    assert_eq!(outcome.old_cell, added.cell);
    assert_eq!(outcome.new_cell, added.cell);
    assert_eq!(
        map.get_creature(guid).unwrap().position(),
        Position::xyz(2.0, 3.0, 4.0)
    );
}
#[test]
fn relocate_map_object_like_cpp_moves_between_cells_in_same_grid() {
    let mut map = test_map();
    let creature = world_object(HighGuid::Creature, 571, 7, false);
    let guid = creature.guid();
    let added = map
        .add_to_map_like_cpp(AccessorObjectKind::Creature, creature)
        .unwrap();
    let new_position = Position::xyz(90.0, 20.0, 5.0);

    let outcome = map
        .relocate_map_object_like_cpp(guid, new_position)
        .unwrap();

    assert!(outcome.relocated);
    assert!(outcome.moved_between_cells);
    assert_eq!(outcome.old_grid, outcome.new_grid);
    assert_eq!(map.get_creature(guid).unwrap().position(), new_position);
    assert_eq!(
        map.get_creature(guid).unwrap().current_cell(),
        Some((
            outcome.new_cell.x_coord % MAX_NUMBER_OF_CELLS,
            outcome.new_cell.y_coord % MAX_NUMBER_OF_CELLS
        ))
    );

    let old_grid = map.get_ngrid(added.grid).unwrap();
    let old_cell = old_grid
        .get_grid_type(
            added.cell.x_coord % MAX_NUMBER_OF_CELLS,
            added.cell.y_coord % MAX_NUMBER_OF_CELLS,
        )
        .unwrap();
    assert!(!old_cell.grid_objects.creatures.contains(&guid));

    let new_cell = old_grid
        .get_grid_type(
            outcome.new_cell.x_coord % MAX_NUMBER_OF_CELLS,
            outcome.new_cell.y_coord % MAX_NUMBER_OF_CELLS,
        )
        .unwrap();
    assert!(new_cell.grid_objects.creatures.contains(&guid));
}
#[test]
fn relocate_map_object_like_cpp_blocks_missing_target_grid_without_panicking() {
    let mut map = test_map();
    let creature = world_object(HighGuid::Creature, 571, 7, false);
    let guid = creature.guid();
    map.insert_map_object(AccessorObjectKind::Creature, creature)
        .unwrap();

    let outcome = map
        .relocate_map_object_like_cpp(guid, Position::xyz(90.0, 20.0, 5.0))
        .unwrap();

    assert!(!outcome.relocated);
    assert!(outcome.blocked_by_unloaded_grid);
    assert_eq!(
        map.get_creature(guid).unwrap().position(),
        Position::xyz(1.0, 2.0, 3.0)
    );
}
#[test]
fn nearby_cell_guids_like_cpp_rejects_invalid_center_without_visits() {
    let map = test_map();
    let nearby = map.nearby_cell_guids_like_cpp(f32::NAN, 0.0, 100.0);

    assert_eq!(nearby.visited_cells, 0);
    assert!(nearby.is_empty());
}
#[test]
fn visit_nearby_cells_of_like_cpp_marks_cells_once_and_collects_objects() {
    let mut map = test_map();
    let player = world_object_with_counter(HighGuid::Player, 1, 571, 7, false);
    let player_guid = player.guid();
    let viewpoint = world_object_with_counter(HighGuid::Creature, 2, 571, 7, false);
    let viewpoint_guid = viewpoint.guid();
    let creature = world_object_with_counter(HighGuid::Creature, 3, 571, 7, false);
    let creature_guid = creature.guid();
    map.add_to_map_like_cpp(AccessorObjectKind::Player, player)
        .unwrap();
    map.add_to_map_like_cpp(AccessorObjectKind::Creature, viewpoint)
        .unwrap();
    map.add_to_map_like_cpp(AccessorObjectKind::Creature, creature)
        .unwrap();

    let plan = map.visit_nearby_cells_of_like_cpp([
        NearbyCellVisitCenter {
            guid: player_guid,
            activation_radius: 0.0,
        },
        NearbyCellVisitCenter {
            guid: viewpoint_guid,
            activation_radius: 0.0,
        },
    ]);

    assert_eq!(plan.marked_cells.len(), 1);
    assert_eq!(plan.nearby.visited_cells, 1);
    assert!(plan.nearby.world.players.contains(&player_guid));
    assert!(plan.nearby.grid.creatures.contains(&viewpoint_guid));
    assert!(plan.nearby.grid.creatures.contains(&creature_guid));
}
#[test]
fn visit_nearby_cells_of_like_cpp_skips_missing_and_invalid_centers() {
    let mut map = test_map();
    let mut invalid_center = world_object_with_counter(HighGuid::Player, 1, 571, 7, false);
    let invalid_guid = invalid_center.guid();
    invalid_center.relocate(Position::xyz(f32::NAN, 0.0, 0.0));
    map.insert_map_object(AccessorObjectKind::Player, invalid_center)
        .unwrap();
    let missing = guid(HighGuid::Player, 9);

    let plan = map.visit_nearby_cells_of_like_cpp([
        NearbyCellVisitCenter {
            guid: invalid_guid,
            activation_radius: 100.0,
        },
        NearbyCellVisitCenter {
            guid: missing,
            activation_radius: 100.0,
        },
    ]);

    assert!(plan.marked_cells.is_empty());
    assert!(plan.nearby.is_empty());
    assert_eq!(plan.skipped_invalid_position_centers, vec![invalid_guid]);
    assert_eq!(plan.skipped_missing_centers, vec![missing]);
}
#[test]
fn delayed_unit_relocation_for_cells_like_cpp_reads_notify_flags_from_map_store() {
    let mut map = test_map();
    let creature_notify = world_object_with_counter(HighGuid::Creature, 1, 571, 7, false);
    let creature_notify_guid = creature_notify.guid();
    let creature_normal = world_object_with_counter(HighGuid::Creature, 2, 571, 7, false);
    let player_notify = world_object_with_counter(HighGuid::Player, 3, 571, 7, false);
    let player_notify_guid = player_notify.guid();
    let player_invalid = world_object_with_counter(HighGuid::Player, 4, 571, 7, false);
    let player_invalid_guid = player_invalid.guid();
    let cell = map
        .add_to_map_like_cpp(AccessorObjectKind::Creature, creature_notify)
        .unwrap()
        .cell;
    map.add_to_map_like_cpp(AccessorObjectKind::Creature, creature_normal)
        .unwrap();
    map.add_to_map_like_cpp(AccessorObjectKind::Player, player_notify)
        .unwrap();
    map.add_to_map_like_cpp(AccessorObjectKind::Player, player_invalid)
        .unwrap();
    for guid in [
        creature_notify_guid,
        player_notify_guid,
        player_invalid_guid,
    ] {
        map.entity_world
            .get_mut(&guid)
            .unwrap()
            .object_mut()
            .object_mut()
            .add_to_notify(ObjectNotifyFlags::VISIBILITY_CHANGED);
    }

    let plan = map.delayed_unit_relocation_for_cells_like_cpp([cell], [player_invalid_guid]);

    assert_eq!(plan.cell_plans.len(), 1);
    assert_eq!(plan.cell_plans[0].cell_coord, cell);
    assert_eq!(
        plan.cell_plans[0].plan.creature_relocations,
        vec![creature_notify_guid]
    );
    assert_eq!(
        plan.cell_plans[0].plan.player_relocations,
        vec![player_notify_guid, player_invalid_guid]
    );
    assert!(
        plan.cell_plans[0]
            .plan
            .skipped_invalid_viewpoints
            .is_empty()
    );
}
#[test]
fn active_objects_near_grid_matches_cpp_cell_range_expansion() {
    let mut map = test_map();
    let coord = GridCoord::new(10, 10);
    map.ensure_grid_created(coord);
    let grid = map.get_ngrid(coord).unwrap();
    assert!(!map.active_objects_near_grid(grid));

    map.mark_active_cell(CellCoord::new(79, 80));
    let grid = map.get_ngrid(coord).unwrap();
    assert!(map.active_objects_near_grid(grid));

    map.unmark_active_cell(CellCoord::new(79, 80));
    map.mark_active_cell(CellCoord::new(1, 1));
    let grid = map.get_ngrid(coord).unwrap();
    assert!(!map.active_objects_near_grid(grid));
}
