//! Movement scenarios for [`super`].
//!
//! Split out of map_tests.rs under #628; assertions and
//! registrations are unchanged and shared fixtures stay in the parent module.

use super::*;

#[test]
fn dynamic_tree_remove_contained_model_empties_tree_and_next_update_early_returns_like_cpp() {
    let mut map = Map::new(1, 0, 0, 60_000);
    let key = dynamic_model_key(45003);
    map.insert_gameobject_model_like_cpp(key);

    let removed = map.remove_gameobject_model_like_cpp(key);
    assert_eq!(
        removed.status,
        DynamicMapTreeModelMutationStatusLikeCpp::Removed
    );
    assert_eq!(removed.model_count_before, 1);
    assert_eq!(removed.model_count_after, 0);
    assert_eq!(removed.unbalanced_before, 1);
    assert_eq!(removed.unbalanced_after, 2);
    assert!(!map.contains_gameobject_model_like_cpp(key));

    let summary = map.update_dynamic_tree_like_cpp(250);
    assert!(summary.empty);
    assert_eq!(summary.timer_before_ms, 200);
    assert_eq!(summary.timer_after_ms, 200);
    assert!(!summary.timer_passed);
    assert_eq!(summary.unbalanced_before, 2);
    assert_eq!(summary.unbalanced_after, 2);
}
#[test]
fn dynamic_tree_missing_remove_is_noop_like_cpp() {
    let mut map = Map::new(1, 0, 0, 60_000);
    let key = dynamic_model_key(45004);

    let missing = map.remove_gameobject_model_like_cpp(key);

    assert_eq!(
        missing.status,
        DynamicMapTreeModelMutationStatusLikeCpp::Missing
    );
    assert_eq!(missing.model_count_before, 0);
    assert_eq!(missing.model_count_after, 0);
    assert_eq!(missing.unbalanced_before, 0);
    assert_eq!(missing.unbalanced_after, 0);
}
#[test]
fn dynamic_tree_contains_reflects_insert_and_remove_like_cpp() {
    let mut map = Map::new(1, 0, 0, 60_000);
    let key = dynamic_model_key(45005);

    assert!(!map.contains_gameobject_model_like_cpp(key));
    map.insert_gameobject_model_like_cpp(key);
    assert!(map.contains_gameobject_model_like_cpp(key));
    map.remove_gameobject_model_like_cpp(key);
    assert!(!map.contains_gameobject_model_like_cpp(key));
}
#[test]
fn map_owned_respawn_add_replace_remove_unload_and_timer_keys_like_cpp() {
    let mut map = test_map();

    assert_eq!(
        map.add_respawn_info_like_cpp(respawn_info(SpawnObjectType::Creature, 10, 100)),
        AddRespawnInfoOutcomeLikeCpp::Inserted
    );
    assert_eq!(
        map.add_respawn_info_like_cpp(respawn_info(SpawnObjectType::Creature, 10, 150)),
        AddRespawnInfoOutcomeLikeCpp::RejectedExistingSoonerOrEqual
    );
    assert_eq!(
        map.get_respawn_time_like_cpp(SpawnObjectType::Creature, 10),
        100
    );
    assert_eq!(
        map.add_respawn_info_like_cpp(respawn_info(SpawnObjectType::Creature, 10, 90)),
        AddRespawnInfoOutcomeLikeCpp::ReplacedExisting
    );
    assert_eq!(
        map.get_respawn_time_like_cpp(SpawnObjectType::Creature, 10),
        90
    );
    assert_eq!(
        map.add_respawn_info_like_cpp(respawn_info(SpawnObjectType::GameObject, 20, 80)),
        AddRespawnInfoOutcomeLikeCpp::Inserted
    );

    let timer_keys = map.respawn_timer_keys_like_cpp().collect::<Vec<_>>();
    assert_eq!(
        timer_keys,
        vec![
            (SpawnObjectType::GameObject, 20),
            (SpawnObjectType::Creature, 10)
        ]
    );

    let removed = map.remove_respawn_time_like_cpp(SpawnObjectType::Creature, 10);
    assert_eq!(removed.map(|info| info.respawn_time), Some(90));
    assert_eq!(
        map.get_respawn_time_like_cpp(SpawnObjectType::Creature, 10),
        0
    );
    assert_eq!(
        map.respawn_timer_keys_like_cpp().collect::<Vec<_>>(),
        vec![(SpawnObjectType::GameObject, 20)]
    );

    map.unload_all_respawn_infos_like_cpp();
    assert_eq!(
        map.get_respawn_time_like_cpp(SpawnObjectType::GameObject, 20),
        0
    );
    assert!(map.respawn_timer_keys_like_cpp().next().is_none());
}
#[test]
fn spawned_pool_data_duplicate_add_and_remove_counter_semantics_like_cpp() {
    let mut pool_data = SpawnedPoolDataLikeCpp::new();

    assert_eq!(
        pool_data.add_spawn_like_cpp(SpawnObjectType::Creature, 101, 7),
        Ok(())
    );
    assert_eq!(
        pool_data.add_spawn_like_cpp(SpawnObjectType::Creature, 101, 7),
        Ok(())
    );
    assert!(pool_data.is_spawned_creature_like_cpp(101));
    assert_eq!(pool_data.get_spawned_objects_like_cpp(7), 2);

    assert_eq!(
        pool_data.remove_spawn_like_cpp(SpawnObjectType::Creature, 101, 7),
        Ok(())
    );
    assert!(!pool_data.is_spawned_creature_like_cpp(101));
    assert_eq!(pool_data.get_spawned_objects_like_cpp(7), 1);

    assert_eq!(
        pool_data.remove_spawn_like_cpp(SpawnObjectType::Creature, 101, 7),
        Ok(())
    );
    assert_eq!(pool_data.get_spawned_objects_like_cpp(7), 0);
    assert_eq!(
        pool_data.remove_spawn_like_cpp(SpawnObjectType::GameObject, 202, 99),
        Ok(())
    );
    assert_eq!(pool_data.get_spawned_objects_like_cpp(99), 0);
}
#[test]
fn process_respawns_pool_remove_respawn_time_action_removes_member_timer_like_cpp() {
    let mut map = test_map();
    let store = SpawnStore::new();
    map.add_respawn_info_like_cpp(respawn_info(SpawnObjectType::GameObject, 75, 200));
    let plan = PoolTypedSpawnPlanLikeCpp {
        kind: PoolMemberKindLikeCpp::GameObject,
        pool_id: 175,
        trigger_from: 0,
        max_limit: Some(1),
        object_plan: Some(PoolSpawnObjectPlanLikeCpp {
            actions: vec![
                PoolSpawnObjectActionLikeCpp::RemoveRespawnTime {
                    kind: PoolMemberKindLikeCpp::GameObject,
                    guid: 75,
                },
                PoolSpawnObjectActionLikeCpp::RemoveRespawnTime {
                    kind: PoolMemberKindLikeCpp::GameObject,
                    guid: 76,
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

    assert_eq!(summary.pool_respawn_timers_removed, 1);
    assert_eq!(summary.pool_respawn_timers_missing, 1);
    assert_eq!(
        map.get_respawn_time_like_cpp(SpawnObjectType::GameObject, 75),
        0
    );
}
#[test]
fn area_trigger_spawn_id_store_remove_desindexes_like_cpp() {
    let mut map = test_map();
    let guid = guid(HighGuid::AreaTrigger, 7101);

    map.insert_map_object_record(
        MapObjectRecord::new_area_trigger(test_area_trigger_for_spawn(71, 7101)).unwrap(),
    )
    .unwrap();
    assert_eq!(map.area_trigger_spawn_id_store_count_like_cpp(71), 1);

    assert!(map.remove_map_object(guid).is_some());
    assert_eq!(map.area_trigger_spawn_id_store_count_like_cpp(71), 0);
    assert!(map.get_area_trigger_by_spawn_id_like_cpp(71).is_none());
}
#[test]
fn check_respawn_like_cpp_allowed_path_preserves_timer_like_cpp() {
    let map = test_map();
    let mut store = SpawnStore::new();
    let group = spawn_group(64, SpawnGroupFlags::NONE);
    store.add_object_spawn(&spawn_data(SpawnObjectType::GameObject, 101, group), |_| {
        false
    });
    let linked = LinkedRespawnStoreLikeCpp::new();
    let mut info = respawn_info(SpawnObjectType::GameObject, 101, 55);

    let outcome =
        map.check_respawn_like_cpp(&mut info, &store, &linked, 1000, 5, false, |_, _| false);

    assert_eq!(outcome, CheckRespawnCompositeOutcomeLikeCpp::Allowed);
    assert_eq!(info.respawn_time, 55);
}
#[test]
fn remove_from_map_like_cpp_removes_store_cell_and_resets_object_binding() {
    let mut map = test_map();
    let creature = world_object(HighGuid::Creature, 571, 7, false);
    let guid = creature.guid();
    let added = map
        .add_to_map_like_cpp(AccessorObjectKind::Creature, creature)
        .unwrap();
    assert!(map.get_creature(guid).is_some());

    let removed = map.remove_from_map_like_cpp(guid, false).unwrap();

    assert_eq!(removed.guid, guid);
    assert_eq!(removed.cell, added.cell);
    assert!(removed.was_in_world);
    assert!(removed.cxx_in_world);
    assert!(!removed.was_active);
    assert!(removed.removed_from_cell);
    assert!(!removed.delete_from_world);
    assert!(map.get_creature(guid).is_none());

    let grid = map.get_ngrid(removed.grid).unwrap();
    let cell = grid
        .get_grid_type(
            removed.cell.x_coord % MAX_NUMBER_OF_CELLS,
            removed.cell.y_coord % MAX_NUMBER_OF_CELLS,
        )
        .unwrap();
    assert!(!cell.grid_objects.creatures.contains(&guid));

    let object = removed.object.unwrap();
    assert!(!object.object().is_in_world());
    assert!(!object.object().is_in_grid());
    assert!(!object.has_current_map());
    assert_eq!(object.current_cell(), None);
}
#[test]
fn remove_from_map_like_cpp_unregisters_personal_phase_tracker_from_object_owner_like_cpp() {
    let mut map = test_map();
    let owner = ObjectGuid::create_player(1, 48401);
    let mut gameobject = test_gameobject_for_spawn(48401, 4840101);
    gameobject
        .world_mut()
        .phase_shift_mut()
        .set_personal_guid_like_cpp(owner);
    let guid = gameobject.world().guid();
    map.add_map_object_record_to_map_like_cpp(
        MapObjectRecord::new_game_object(gameobject).unwrap(),
    )
    .unwrap();
    map.register_personal_phase_object_for_test(84, owner, guid);
    assert_eq!(map.personal_phase_tracker().tracker_count(), 1);

    let outcome = map.remove_from_map_like_cpp(guid, false).unwrap();

    assert_eq!(outcome.personal_phase_unregister.phase_owner, owner);
    assert!(outcome.personal_phase_unregister.attempted);
    assert!(outcome.personal_phase_unregister.tracker_found);
    assert!(outcome.personal_phase_unregister.removed);
    assert!(outcome.personal_phase_unregister.removed_owner_tracker);
    assert_eq!(map.personal_phase_tracker().tracker_count(), 0);
}
#[test]
fn remove_from_map_like_cpp_personal_phase_empty_or_missing_owner_noops_like_cpp() {
    let mut empty_owner_map = test_map();
    let empty_owner_gameobject = test_gameobject_for_spawn(48402, 4840201);
    let empty_owner_guid = empty_owner_gameobject.world().guid();
    empty_owner_map
        .add_map_object_record_to_map_like_cpp(
            MapObjectRecord::new_game_object(empty_owner_gameobject).unwrap(),
        )
        .unwrap();

    let empty_owner = empty_owner_map
        .remove_from_map_like_cpp(empty_owner_guid, false)
        .unwrap();
    assert_eq!(
        empty_owner.personal_phase_unregister.phase_owner,
        ObjectGuid::EMPTY
    );
    assert!(!empty_owner.personal_phase_unregister.attempted);
    assert!(!empty_owner.personal_phase_unregister.tracker_found);
    assert!(!empty_owner.personal_phase_unregister.removed);

    let mut missing_tracker_map = test_map();
    let missing_owner = ObjectGuid::create_player(1, 48403);
    let mut missing_tracker_gameobject = test_gameobject_for_spawn(48403, 4840301);
    missing_tracker_gameobject
        .world_mut()
        .phase_shift_mut()
        .set_personal_guid_like_cpp(missing_owner);
    let missing_tracker_guid = missing_tracker_gameobject.world().guid();
    missing_tracker_map
        .add_map_object_record_to_map_like_cpp(
            MapObjectRecord::new_game_object(missing_tracker_gameobject).unwrap(),
        )
        .unwrap();

    let missing_tracker = missing_tracker_map
        .remove_from_map_like_cpp(missing_tracker_guid, false)
        .unwrap();
    assert_eq!(
        missing_tracker.personal_phase_unregister.phase_owner,
        missing_owner
    );
    assert!(missing_tracker.personal_phase_unregister.attempted);
    assert!(!missing_tracker.personal_phase_unregister.tracker_found);
    assert!(!missing_tracker.personal_phase_unregister.removed);
}
#[test]
fn remove_from_map_like_cpp_visibility_on_destroy_follows_cpp_in_world_type_range() {
    let mut dynamic_map = test_map();
    let dynamic = test_dynamic_object_for_viewpoint(4840401);
    let dynamic_guid = dynamic.world().guid();
    dynamic_map
        .insert_map_object_record(MapObjectRecord::new_dynamic_object(dynamic).unwrap())
        .unwrap();
    let dynamic_removed = dynamic_map
        .remove_from_map_like_cpp(dynamic_guid, false)
        .unwrap();
    assert!(dynamic_removed.was_in_world);
    assert!(!dynamic_removed.cxx_in_world);
    assert!(
        dynamic_removed
            .visibility_on_destroy
            .update_object_visibility_on_destroy_represented
    );

    let mut area_map = test_map();
    let area_trigger = test_area_trigger_for_spawn(48405, 4840501);
    let area_guid = area_trigger.world().guid();
    area_map
        .insert_map_object_record(MapObjectRecord::new_area_trigger(area_trigger).unwrap())
        .unwrap();
    let area_removed = area_map.remove_from_map_like_cpp(area_guid, false).unwrap();
    assert!(area_removed.was_in_world);
    assert!(!area_removed.cxx_in_world);
    assert!(
        area_removed
            .visibility_on_destroy
            .update_object_visibility_on_destroy_represented
    );
}
#[test]
fn remove_from_map_like_cpp_visibility_on_destroy_skips_in_world_eligible_records() {
    let mut gameobject_map = test_map();
    let gameobject = test_gameobject_for_spawn(48406, 4840601);
    let gameobject_guid = gameobject.world().guid();
    gameobject_map
        .add_map_object_record_to_map_like_cpp(
            MapObjectRecord::new_game_object(gameobject).unwrap(),
        )
        .unwrap();
    let gameobject_removed = gameobject_map
        .remove_from_map_like_cpp(gameobject_guid, false)
        .unwrap();
    assert!(gameobject_removed.cxx_in_world);
    assert!(
        !gameobject_removed
            .visibility_on_destroy
            .update_object_visibility_on_destroy_represented
    );

    let mut creature_map = test_map();
    let creature = test_creature_for_spawn(48407, 4840701, true);
    let creature_guid = creature.guid();
    creature_map
        .add_map_object_record_to_map_like_cpp(MapObjectRecord::new_creature(creature).unwrap())
        .unwrap();
    let creature_removed = creature_map
        .remove_from_map_like_cpp(creature_guid, false)
        .unwrap();
    assert!(creature_removed.cxx_in_world);
    assert!(
        !creature_removed
            .visibility_on_destroy
            .update_object_visibility_on_destroy_represented
    );

    let mut player_map = test_map();
    let player = test_player_for_viewpoint(4840801);
    let player_guid = player.guid();
    player_map
        .add_map_object_record_to_map_like_cpp(MapObjectRecord::new_player(player).unwrap())
        .unwrap();
    let player_removed = player_map
        .remove_from_map_like_cpp(player_guid, false)
        .unwrap();
    assert!(player_removed.cxx_in_world);
    assert!(
        !player_removed
            .visibility_on_destroy
            .update_object_visibility_on_destroy_represented
    );

    let mut pet_map = test_map();
    let pet = test_pet(4840901, true);
    let pet_guid = pet.creature().guid();
    pet_map
        .add_map_object_record_to_map_like_cpp(MapObjectRecord::new_pet(pet).unwrap())
        .unwrap();
    let pet_removed = pet_map.remove_from_map_like_cpp(pet_guid, false).unwrap();
    assert!(pet_removed.cxx_in_world);
    assert!(
        !pet_removed
            .visibility_on_destroy
            .update_object_visibility_on_destroy_represented
    );

    let mut transport_map = test_map();
    let transport = test_transport(4841001, true);
    let transport_guid = transport.world().guid();
    transport_map
        .add_map_object_record_to_map_like_cpp(MapObjectRecord::new_transport(transport).unwrap())
        .unwrap();
    let transport_removed = transport_map
        .remove_from_map_like_cpp(transport_guid, false)
        .unwrap();
    assert!(transport_removed.cxx_in_world);
    assert!(
        !transport_removed
            .visibility_on_destroy
            .update_object_visibility_on_destroy_represented
    );
}
#[test]
fn remove_list_duplicate_enqueue_follows_cpp_cleanup_before_set_insert_like_cpp() {
    let mut map = test_map();
    let mut creature = test_creature_for_spawn(41903, 4190301, true);
    let guid = creature.guid();
    creature
        .unit_mut()
        .world_mut()
        .object_mut()
        .remove_from_world();
    map.add_map_object_record_to_map_like_cpp(MapObjectRecord::new_creature(creature).unwrap())
        .unwrap();

    let first = map.add_object_to_remove_list_like_cpp(guid);
    let second = map.add_object_to_remove_list_like_cpp(guid);

    assert!(first.queued);
    assert!(second.duplicate);
    assert_eq!(map.objects_to_remove_count_like_cpp(), 1);
    assert_eq!(
        map.get_typed_creature(guid)
            .unwrap()
            .cleanup_before_delete_count(),
        2
    );
}
#[test]
fn remove_list_drain_missing_stale_guid_does_not_create_object_like_cpp() {
    let mut map = test_map();
    let guid = guid(HighGuid::Creature, 4190401);
    map.enqueue_object_to_remove_for_test(guid);

    let outcome = map.remove_all_objects_in_remove_list_like_cpp();

    assert_eq!(outcome.processed, 1);
    assert_eq!(outcome.missing_or_stale, 1);
    assert_eq!(outcome.removed, 0);
    assert_eq!(map.objects_to_remove_count_like_cpp(), 0);
    assert_eq!(map.map_object_count(), 0);
}
#[test]
fn remove_from_map_like_cpp_dynamic_object_not_viewpoint_skips_cleanup_like_cpp() {
    let mut map = test_map();
    let mut player = test_player_for_viewpoint(4270301);
    let player_guid = player.guid();
    let mut dynamic_object = test_dynamic_object_for_viewpoint(4270302);
    let dynamic_object_guid = dynamic_object.world().guid();
    player.set_farsight_object_like_cpp(dynamic_object_guid);
    dynamic_object.set_caster_guid(player_guid);
    dynamic_object.bind_to_caster(player_guid);
    map.insert_map_object_record(MapObjectRecord::new_player(player).unwrap())
        .unwrap();
    map.insert_map_object_record(MapObjectRecord::new_dynamic_object(dynamic_object).unwrap())
        .unwrap();

    let removed = map
        .remove_from_map_like_cpp(dynamic_object_guid, true)
        .unwrap();

    assert_eq!(removed.dynamic_object_caster_viewpoint, None);
    assert_eq!(
        map.get_typed_player(player_guid)
            .unwrap()
            .active_data()
            .farsight_object,
        dynamic_object_guid
    );
    assert!(map.map_object_record(dynamic_object_guid).is_none());
}
#[test]
fn remove_from_map_like_cpp_dynamic_object_not_in_world_skips_viewpoint_cleanup_like_cpp() {
    let mut map = test_map();
    let mut player = test_player_for_viewpoint(4270401);
    let player_guid = player.guid();
    let mut dynamic_object = test_dynamic_object_for_viewpoint(4270402);
    let dynamic_object_guid = dynamic_object.world().guid();
    player.set_farsight_object_like_cpp(dynamic_object_guid);
    dynamic_object.set_caster_guid(player_guid);
    dynamic_object.bind_to_caster(player_guid);
    dynamic_object.set_caster_viewpoint();
    dynamic_object.world_mut().object_mut().remove_from_world();
    map.insert_map_object_record(MapObjectRecord::new_player(player).unwrap())
        .unwrap();
    map.insert_map_object_record(MapObjectRecord::new_dynamic_object(dynamic_object).unwrap())
        .unwrap();

    let removed = map
        .remove_from_map_like_cpp(dynamic_object_guid, true)
        .unwrap();

    assert_eq!(removed.dynamic_object_caster_viewpoint, None);
    assert!(!removed.was_in_world);
    assert_eq!(
        map.get_typed_player(player_guid)
            .unwrap()
            .active_data()
            .farsight_object,
        dynamic_object_guid
    );
    assert!(map.map_object_record(dynamic_object_guid).is_none());
}
#[test]
fn move_list_add_same_guid_updates_position_without_duplicate_like_cpp() {
    let mut map = test_map();
    let creature = test_creature_for_spawn(44101, 4410101, true);
    let guid = creature.guid();
    map.add_map_object_record_to_map_like_cpp(MapObjectRecord::new_creature(creature).unwrap())
        .unwrap();
    let first = Position::xyz(2.0, 3.0, 4.0);
    let second = Position::xyz(3.0, 4.0, 5.0);

    assert_eq!(
        map.add_creature_to_move_list_like_cpp(guid, first),
        AddObjectToMoveListOutcomeLikeCpp::Queued
    );
    assert_eq!(
        map.add_creature_to_move_list_like_cpp(guid, second),
        AddObjectToMoveListOutcomeLikeCpp::UpdatedExisting
    );

    assert_eq!(
        map.move_list_len_like_cpp(MapObjectMoveListFamilyLikeCpp::Creature),
        1
    );
    let pending = map
        .pending_cell_move_like_cpp(MapObjectMoveListFamilyLikeCpp::Creature, guid)
        .unwrap();
    assert_eq!(pending.state, MapObjectCellMoveStateLikeCpp::Active);
    assert_eq!(pending.new_position, second);
}
#[test]
fn move_list_remove_marks_inactive_and_drain_resets_without_relocation_like_cpp() {
    let mut map = test_map();
    let creature = test_creature_for_spawn(44102, 4410201, true);
    let guid = creature.guid();
    let original_position = creature.unit().world().position();
    map.add_map_object_record_to_map_like_cpp(MapObjectRecord::new_creature(creature).unwrap())
        .unwrap();

    assert_eq!(
        map.add_creature_to_move_list_like_cpp(guid, Position::xyz(50.0, 50.0, 6.0)),
        AddObjectToMoveListOutcomeLikeCpp::Queued
    );
    assert_eq!(
        map.remove_creature_from_move_list_like_cpp(guid),
        RemoveObjectFromMoveListOutcomeLikeCpp::MarkedInactive
    );
    let summary = map.move_all_creatures_in_move_list_like_cpp();

    assert_eq!(summary.processed, 1);
    assert_eq!(summary.inactive_reset, 1);
    assert_eq!(summary.relocated, 0);
    assert_eq!(
        map.pending_cell_move_like_cpp(MapObjectMoveListFamilyLikeCpp::Creature, guid),
        None
    );
    assert_eq!(map.map_object(guid).unwrap().position(), original_position);
}
#[test]
fn move_list_drain_active_in_world_relocates_cell_membership_like_cpp() {
    let mut map = test_map();
    let creature = test_creature_for_spawn(44103, 4410301, true);
    let guid = creature.guid();
    map.add_map_object_record_to_map_like_cpp(MapObjectRecord::new_creature(creature).unwrap())
        .unwrap();
    let new_position = Position::xyz(120.0, 120.0, 7.0);
    map.load_grid(new_position.x, new_position.y);
    let new_cell = Cell::from_world(new_position.x, new_position.y);

    assert_eq!(
        map.add_creature_to_move_list_like_cpp(guid, new_position),
        AddObjectToMoveListOutcomeLikeCpp::Queued
    );
    let summary = map.move_all_creatures_in_move_list_like_cpp();

    assert_eq!(summary.processed, 1);
    assert_eq!(summary.relocated, 1);
    let stored = map.map_object(guid).unwrap();
    assert_eq!(stored.position(), new_position);
    assert_eq!(
        stored.current_cell(),
        Some((new_cell.cell_x(), new_cell.cell_y()))
    );
    let nearby = map.exact_cell_guids_like_cpp(new_cell.cell_coord());
    assert!(nearby.grid.creatures.contains(&guid) || nearby.world.creatures.contains(&guid));
}
#[test]
fn move_list_drain_tolerates_missing_wrong_kind_and_not_in_world_like_cpp() {
    let mut map = test_map();
    let missing_guid = guid(HighGuid::Creature, 4410401);
    let gameobject = test_gameobject_for_spawn(44104, 4410402);
    let gameobject_guid = gameobject.world().guid();
    map.add_map_object_record_to_map_like_cpp(
        MapObjectRecord::new_game_object(gameobject).unwrap(),
    )
    .unwrap();
    let mut not_in_world = test_creature_for_spawn(44105, 4410403, true);
    not_in_world
        .unit_mut()
        .world_mut()
        .object_mut()
        .remove_from_world();
    let not_in_world_guid = not_in_world.guid();
    map.insert_map_object_record(MapObjectRecord::new_creature(not_in_world).unwrap())
        .unwrap();

    assert_eq!(
        map.add_creature_to_move_list_like_cpp(missing_guid, Position::xyz(2.0, 2.0, 2.0)),
        AddObjectToMoveListOutcomeLikeCpp::MissingOrStale
    );
    assert!(matches!(
        map.add_creature_to_move_list_like_cpp(gameobject_guid, Position::xyz(2.0, 2.0, 2.0)),
        AddObjectToMoveListOutcomeLikeCpp::WrongKind {
            actual: AccessorObjectKind::GameObject
        }
    ));
    map.creatures_to_move.push(missing_guid);
    map.creature_move_states.insert(
        missing_guid,
        PendingCellMoveLikeCpp {
            state: MapObjectCellMoveStateLikeCpp::Active,
            new_position: Position::xyz(2.0, 2.0, 2.0),
        },
    );
    map.creatures_to_move.push(gameobject_guid);
    map.creature_move_states.insert(
        gameobject_guid,
        PendingCellMoveLikeCpp {
            state: MapObjectCellMoveStateLikeCpp::Active,
            new_position: Position::xyz(2.0, 2.0, 2.0),
        },
    );
    assert_eq!(
        map.add_creature_to_move_list_like_cpp(not_in_world_guid, Position::xyz(2.0, 2.0, 2.0)),
        AddObjectToMoveListOutcomeLikeCpp::Queued
    );

    let summary = map.move_all_creatures_in_move_list_like_cpp();

    assert_eq!(summary.processed, 3);
    assert_eq!(summary.missing_or_stale, 1);
    assert_eq!(summary.wrong_kind, 1);
    assert_eq!(summary.not_in_world, 1);
    assert_eq!(
        map.move_list_len_like_cpp(MapObjectMoveListFamilyLikeCpp::Creature),
        0
    );
    assert_eq!(
        map.pending_cell_move_like_cpp(MapObjectMoveListFamilyLikeCpp::Creature, missing_guid),
        None
    );
}
#[test]
fn move_list_dynamic_and_area_trigger_blocked_unloaded_grid_do_not_queue_remove_like_cpp() {
    let mut map = test_map();
    let mut dynamic_object = test_dynamic_object_for_viewpoint(4410501);
    dynamic_object.world_mut().set_active(false);
    let dynamic_guid = dynamic_object.world().guid();
    map.add_map_object_record_to_map_like_cpp(
        MapObjectRecord::new_dynamic_object(dynamic_object).unwrap(),
    )
    .unwrap();
    let area_trigger = test_area_trigger_for_update(4410502, 10_000, true);
    let area_guid = area_trigger.world().guid();
    map.add_map_object_record_to_map_like_cpp(
        MapObjectRecord::new_area_trigger(area_trigger).unwrap(),
    )
    .unwrap();
    let unloaded_grid_position = Position::xyz(5_000.0, 5_000.0, 1.0);

    assert_eq!(
        map.add_dynamic_object_to_move_list_like_cpp(dynamic_guid, unloaded_grid_position),
        AddObjectToMoveListOutcomeLikeCpp::Queued
    );
    assert_eq!(
        map.add_area_trigger_to_move_list_like_cpp(area_guid, unloaded_grid_position),
        AddObjectToMoveListOutcomeLikeCpp::Queued
    );
    let dyn_summary = map.move_all_dynamic_objects_in_move_list_like_cpp();
    let area_summary = map.move_all_area_triggers_in_move_list_like_cpp();

    assert_eq!(dyn_summary.blocked_by_unloaded_grid, 1);
    assert_eq!(area_summary.blocked_by_unloaded_grid, 1);
    assert_eq!(dyn_summary.remove_list_queued, 0);
    assert_eq!(area_summary.remove_list_queued, 0);
    assert_eq!(map.objects_to_remove_count_like_cpp(), 0);
    assert!(map.map_object_record(dynamic_guid).is_some());
    assert!(map.map_object_record(area_guid).is_some());
}
#[test]
fn unload_grid_at_true_does_not_drain_move_lists_and_unload_all_helper_clears_cpp_subset_like_cpp()
{
    let mut map = test_map();
    let creature = test_creature_for_spawn(44107, 4410701, true);
    let creature_guid = creature.guid();
    let creature_original_position = creature.unit().world().position();
    let gameobject = test_gameobject_for_spawn(44107, 4410702);
    let gameobject_guid = gameobject.world().guid();
    let gameobject_original_position = gameobject.world().position();
    let area_trigger = test_area_trigger_for_update(4410703, 10_000, true);
    let area_guid = area_trigger.world().guid();
    let area_original_position = area_trigger.world().position();

    map.add_map_object_record_to_map_like_cpp(MapObjectRecord::new_creature(creature).unwrap())
        .unwrap();
    map.add_map_object_record_to_map_like_cpp(
        MapObjectRecord::new_game_object(gameobject).unwrap(),
    )
    .unwrap();
    map.add_map_object_record_to_map_like_cpp(
        MapObjectRecord::new_area_trigger(area_trigger).unwrap(),
    )
    .unwrap();
    let pending_position = Position::xyz(2.0, 3.0, 4.0);
    assert_eq!(
        map.add_creature_to_move_list_like_cpp(creature_guid, pending_position),
        AddObjectToMoveListOutcomeLikeCpp::Queued
    );
    assert_eq!(
        map.add_game_object_to_move_list_like_cpp(gameobject_guid, pending_position),
        AddObjectToMoveListOutcomeLikeCpp::Queued
    );
    assert_eq!(
        map.add_area_trigger_to_move_list_like_cpp(area_guid, pending_position),
        AddObjectToMoveListOutcomeLikeCpp::Queued
    );

    let unload_position = Position::xyz(3_500.0, 3_500.0, 0.0);
    map.load_grid(unload_position.x, unload_position.y);
    let unload_cell = Cell::from_world(unload_position.x, unload_position.y);
    let unload_grid = GridCoord::new(unload_cell.grid_x(), unload_cell.grid_y());

    assert!(map.unload_grid_at(unload_grid, true));

    assert_eq!(
        map.map_object(creature_guid).unwrap().position(),
        creature_original_position
    );
    assert_eq!(
        map.map_object(gameobject_guid).unwrap().position(),
        gameobject_original_position
    );
    assert_eq!(
        map.map_object(area_guid).unwrap().position(),
        area_original_position
    );
    assert_eq!(
        map.move_list_len_like_cpp(MapObjectMoveListFamilyLikeCpp::Creature),
        1
    );
    assert_eq!(
        map.move_list_len_like_cpp(MapObjectMoveListFamilyLikeCpp::GameObject),
        1
    );
    assert_eq!(
        map.move_list_len_like_cpp(MapObjectMoveListFamilyLikeCpp::AreaTrigger),
        1
    );
    assert_eq!(
        map.pending_cell_move_like_cpp(MapObjectMoveListFamilyLikeCpp::Creature, creature_guid)
            .unwrap()
            .new_position,
        pending_position
    );
    assert_eq!(map.lifecycle().evacuates, 0);

    map.clear_unload_all_delayed_moves_like_cpp();

    assert_eq!(
        map.move_list_len_like_cpp(MapObjectMoveListFamilyLikeCpp::Creature),
        0
    );
    assert_eq!(
        map.move_list_len_like_cpp(MapObjectMoveListFamilyLikeCpp::GameObject),
        0
    );
    assert_eq!(
        map.move_list_len_like_cpp(MapObjectMoveListFamilyLikeCpp::AreaTrigger),
        1
    );
    assert_eq!(
        map.pending_cell_move_like_cpp(MapObjectMoveListFamilyLikeCpp::Creature, creature_guid),
        None
    );
    assert_eq!(
        map.pending_cell_move_like_cpp(MapObjectMoveListFamilyLikeCpp::GameObject, gameobject_guid),
        None
    );
    assert!(
        map.pending_cell_move_like_cpp(MapObjectMoveListFamilyLikeCpp::AreaTrigger, area_guid)
            .is_some()
    );
}
#[test]
fn area_trigger_update_expiry_queues_remove_list_and_preserves_record_like_cpp() {
    let mut map = test_map();
    let area_trigger = test_area_trigger_for_update(4340201, 250, true);
    let area_trigger_guid = area_trigger.world().guid();
    map.insert_map_object_record(MapObjectRecord::new_area_trigger(area_trigger).unwrap())
        .unwrap();

    let outcome = map.update_area_trigger_like_cpp(area_trigger_guid, 250);

    assert_eq!(
        outcome.status,
        AreaTriggerUpdateStatusLikeCpp::ExpiredRemoveQueued
    );
    assert_eq!(outcome.duration_before_ms, Some(250));
    assert_eq!(outcome.duration_after_ms, Some(250));
    assert_eq!(outcome.time_since_created_before_ms, Some(0));
    assert_eq!(outcome.time_since_created_after_ms, Some(250));
    assert!(outcome.non_static_movement_would_run);
    assert!(!outcome.ai_update_would_run);
    assert!(!outcome.target_list_update_would_run);
    assert_eq!(outcome.remove_list.unwrap().queued, true);
    assert_eq!(map.objects_to_remove_count_like_cpp(), 1);
    let area_trigger = map
        .map_object_record(area_trigger_guid)
        .unwrap()
        .area_trigger()
        .unwrap();
    assert!(area_trigger.is_removed());
}
