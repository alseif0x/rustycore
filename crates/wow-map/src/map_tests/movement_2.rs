//! Movement scenarios for [`super`].
//!
//! Split out of map_tests.rs under #628; assertions and
//! registrations are unchanged and shared fixtures stay in the parent module.

use super::*;

#[test]
fn scene_object_update_missing_creator_queues_remove_and_preserves_record_like_cpp() {
    let mut map = test_map();
    let scene_object = test_scene_object_for_update(4370201, true, ObjectGuid::EMPTY);
    let scene_object_guid = scene_object.world().guid();
    map.insert_map_object_record(MapObjectRecord::new_scene_object(scene_object).unwrap())
        .unwrap();

    let outcome = map.update_scene_object_like_cpp(
        scene_object_guid,
        250,
        SceneObjectUpdateContextLikeCpp {
            creator_exists: false,
            linked_aura_exists: true,
        },
    );

    assert_eq!(outcome.status, SceneObjectUpdateStatusLikeCpp::RemoveQueued);
    assert!(outcome.world_update_would_run);
    assert!(outcome.should_be_removed);
    assert_eq!(outcome.remove_list.unwrap().queued, true);
    assert_eq!(map.objects_to_remove_count_like_cpp(), 1);
    assert!(map.map_object_record(scene_object_guid).is_some());
}
#[test]
fn conversation_update_expiry_queues_remove_list_and_preserves_record_like_cpp() {
    let mut map = test_map();
    let conversation = test_conversation_for_update(4360201, 250, true);
    let conversation_guid = conversation.world().guid();
    map.insert_map_object_record(MapObjectRecord::new_conversation(conversation).unwrap())
        .unwrap();

    let outcome = map.update_conversation_like_cpp(conversation_guid, 250);

    assert_eq!(
        outcome.status,
        ConversationUpdateStatusLikeCpp::ExpiredRemoveQueued
    );
    assert_eq!(outcome.duration_before_ms, Some(250));
    assert_eq!(outcome.duration_after_ms, Some(250));
    assert!(outcome.script_update_would_run);
    assert!(!outcome.world_update_would_run);
    assert_eq!(outcome.remove_list.unwrap().queued, true);
    assert_eq!(map.objects_to_remove_count_like_cpp(), 1);
    let conversation = map
        .map_object_record(conversation_guid)
        .unwrap()
        .conversation()
        .unwrap();
    assert!(conversation.is_removed());
}
#[test]
fn dynamic_object_update_expired_then_remove_list_drain_clears_farsight_like_cpp() {
    let mut map = test_map();
    let player = test_player_for_viewpoint(4290301);
    let player_guid = player.guid();
    map.insert_map_object_record(MapObjectRecord::new_player(player).unwrap())
        .unwrap();
    let create = create_farsight_focus_for_tests(&mut map, player_guid);
    assert_eq!(
        create.status,
        FarsightDynamicObjectCreateStatusLikeCpp::Created
    );
    let dynamic_object_guid = create.dynamic_object_guid.unwrap();
    map.get_typed_dynamic_object_mut(dynamic_object_guid)
        .unwrap()
        .set_duration(1);
    assert_eq!(
        map.get_typed_player(player_guid)
            .unwrap()
            .active_data()
            .farsight_object,
        dynamic_object_guid
    );
    assert!(
        map.get_typed_dynamic_object(dynamic_object_guid)
            .unwrap()
            .is_caster_viewpoint()
    );

    let update = map.update_dynamic_object_like_cpp(dynamic_object_guid, 1);

    assert_eq!(
        update.status,
        DynamicObjectUpdateStatusLikeCpp::ExpiredRemoveQueued
    );
    assert_eq!(map.objects_to_remove_count_like_cpp(), 1);
    assert!(map.map_object_record(dynamic_object_guid).is_some());
    assert_eq!(
        map.get_typed_player(player_guid)
            .unwrap()
            .active_data()
            .farsight_object,
        dynamic_object_guid
    );

    let drain = map.remove_all_objects_in_remove_list_like_cpp();

    assert_eq!(drain.processed, 1);
    assert_eq!(drain.removed, 1);
    assert_eq!(map.objects_to_remove_count_like_cpp(), 0);
    assert!(map.map_object_record(dynamic_object_guid).is_none());
    assert_eq!(
        map.get_typed_player(player_guid)
            .unwrap()
            .active_data()
            .farsight_object,
        ObjectGuid::EMPTY
    );
}
#[test]
fn player_set_viewpoint_remove_last_viewer_consumes_set_world_object_like_cpp() {
    let mut map = test_map();
    let mut player = test_player_for_viewpoint(4240301);
    let player_guid = player.guid();
    let (target_guid, _cell, _grid) =
        add_loaded_grid_creature_for_switch(&mut map, 424030, 4240302);
    player.set_farsight_object_like_cpp(target_guid);
    map.insert_map_object_record(MapObjectRecord::new_player(player).unwrap())
        .unwrap();
    map.get_typed_creature_mut(target_guid)
        .unwrap()
        .unit_mut()
        .add_player_to_vision_like_cpp(player_guid);
    assert_eq!(map.pending_switch_like_cpp(target_guid), None);

    let outcome =
        map.apply_player_set_viewpoint_unit_like_cpp(player_guid, target_guid, false, None);

    assert_eq!(outcome.status, PlayerSetViewpointStatusLikeCpp::Removed);
    assert!(!outcome.update_visibility_requested);
    assert!(outcome.set_seer_requested);
    assert_eq!(
        outcome.set_world_object,
        Some(SetWorldObjectOutcomeLikeCpp {
            guid: target_guid,
            on: false,
            status: SetWorldObjectStatusLikeCpp::Delegated(
                AddObjectToSwitchListStatusLikeCpp::Queued
            ),
        })
    );
    assert_eq!(
        map.get_typed_player(player_guid)
            .unwrap()
            .active_data()
            .farsight_object,
        ObjectGuid::EMPTY
    );
    assert!(
        map.get_typed_creature(target_guid)
            .unwrap()
            .unit()
            .subsystems()
            .control
            .shared_vision_guids
            .is_empty()
    );
    assert_eq!(map.pending_switch_like_cpp(target_guid), Some(false));
}
#[test]
fn player_set_viewpoint_remove_mismatch_is_no_mutation_like_cpp() {
    let mut map = test_map();
    let mut player = test_player_for_viewpoint(4240401);
    let player_guid = player.guid();
    let (target_guid, _cell, _grid) =
        add_loaded_grid_creature_for_switch(&mut map, 424040, 4240402);
    let existing_guid = guid(HighGuid::Creature, 4240409);
    player.set_farsight_object_like_cpp(existing_guid);
    map.insert_map_object_record(MapObjectRecord::new_player(player).unwrap())
        .unwrap();

    let outcome =
        map.apply_player_set_viewpoint_unit_like_cpp(player_guid, target_guid, false, None);

    assert_eq!(
        outcome.status,
        PlayerSetViewpointStatusLikeCpp::ViewpointMismatch
    );
    assert_eq!(outcome.set_world_object, None);
    assert_eq!(
        map.get_typed_player(player_guid)
            .unwrap()
            .active_data()
            .farsight_object,
        existing_guid
    );
    assert!(
        map.get_typed_creature(target_guid)
            .unwrap()
            .unit()
            .subsystems()
            .control
            .shared_vision_guids
            .is_empty()
    );
    assert_eq!(map.pending_switch_like_cpp(target_guid), None);
}
#[test]
fn player_remove_from_world_viewpoint_dynamic_object_cleans_before_extract_like_cpp() {
    let mut map = test_map();
    let mut player = test_player_for_viewpoint(4930101);
    let player_guid = player.guid();
    let mut dynamic_object = test_dynamic_object_for_viewpoint(4930102);
    let dynamic_object_guid = dynamic_object.world().guid();
    player.set_farsight_object_like_cpp(dynamic_object_guid);
    dynamic_object.set_caster_guid(player_guid);
    dynamic_object.bind_to_caster(player_guid);
    dynamic_object.set_caster_viewpoint();
    map.insert_map_object_record(MapObjectRecord::new_player(player).unwrap())
        .unwrap();
    map.insert_map_object_record(MapObjectRecord::new_dynamic_object(dynamic_object).unwrap())
        .unwrap();

    let removed = map.remove_from_map_like_cpp(player_guid, false).unwrap();

    let cleanup = removed.player_viewpoint_cleanup.unwrap();
    assert_eq!(cleanup.player_guid, player_guid);
    assert_eq!(cleanup.viewpoint_guid, dynamic_object_guid);
    assert_eq!(
        cleanup.status,
        PlayerRemoveFromWorldViewpointCleanupStatusLikeCpp::RemovedDynamicObjectViewpoint
    );
    assert!(!cleanup.update_visibility_requested);
    assert!(cleanup.set_seer_requested);
    assert!(!cleanup.object_accessor_fanout_represented);
    assert_eq!(cleanup.dynamic_object_caster_viewpoint, None);
    let player_set_viewpoint = cleanup.player_set_viewpoint.unwrap();
    assert_eq!(player_set_viewpoint.player_guid, player_guid);
    assert_eq!(player_set_viewpoint.target_guid, dynamic_object_guid);
    assert!(!player_set_viewpoint.apply);
    assert_eq!(
        player_set_viewpoint.status,
        PlayerSetViewpointStatusLikeCpp::Removed
    );
    assert!(!player_set_viewpoint.update_visibility_requested);
    assert!(player_set_viewpoint.set_seer_requested);
    assert!(map.map_object_record(player_guid).is_none());
    assert!(map.map_object_record(dynamic_object_guid).is_some());
    assert!(
        map.get_typed_dynamic_object(dynamic_object_guid)
            .unwrap()
            .is_caster_viewpoint()
    );
    assert!(!removed.object.unwrap().object().is_in_world());
}
#[test]
fn player_remove_from_world_viewpoint_missing_or_unsupported_target_no_cleanup_success_like_cpp() {
    let mut missing_map = test_map();
    let mut missing_player = test_player_for_viewpoint(4930301);
    let missing_player_guid = missing_player.guid();
    let missing_viewpoint_guid = guid(HighGuid::Creature, 4930302);
    missing_player.set_farsight_object_like_cpp(missing_viewpoint_guid);
    missing_map
        .insert_map_object_record(MapObjectRecord::new_player(missing_player).unwrap())
        .unwrap();

    let missing_removed = missing_map
        .remove_from_map_like_cpp(missing_player_guid, false)
        .unwrap();

    let missing_cleanup = missing_removed.player_viewpoint_cleanup.unwrap();
    assert_eq!(
        missing_cleanup.status,
        PlayerRemoveFromWorldViewpointCleanupStatusLikeCpp::MissingTarget
    );
    assert_eq!(missing_cleanup.player_set_viewpoint, None);
    assert_eq!(missing_cleanup.dynamic_object_caster_viewpoint, None);
    assert!(missing_map.map_object_record(missing_player_guid).is_none());

    let mut unsupported_map = test_map();
    let mut unsupported_player = test_player_for_viewpoint(4930401);
    let unsupported_player_guid = unsupported_player.guid();
    let game_object = game_object_with_counter(4930402, 571, 7, true);
    let game_object_guid = game_object.world().guid();
    unsupported_player.set_farsight_object_like_cpp(game_object_guid);
    unsupported_map
        .insert_map_object_record(MapObjectRecord::new_player(unsupported_player).unwrap())
        .unwrap();
    unsupported_map
        .insert_map_object_record(MapObjectRecord::new_game_object(game_object).unwrap())
        .unwrap();

    let unsupported_removed = unsupported_map
        .remove_from_map_like_cpp(unsupported_player_guid, false)
        .unwrap();

    let unsupported_cleanup = unsupported_removed.player_viewpoint_cleanup.unwrap();
    assert_eq!(
        unsupported_cleanup.status,
        PlayerRemoveFromWorldViewpointCleanupStatusLikeCpp::TargetNotSeer
    );
    assert_eq!(unsupported_cleanup.player_set_viewpoint, None);
    assert_eq!(unsupported_cleanup.dynamic_object_caster_viewpoint, None);
    assert!(
        unsupported_map
            .map_object_record(unsupported_player_guid)
            .is_none()
    );
    assert!(
        unsupported_map
            .map_object_record(game_object_guid)
            .is_some()
    );
}
#[test]
fn player_remove_from_world_not_in_world_or_empty_farsight_emits_no_cleanup_like_cpp() {
    let mut not_in_world_map = test_map();
    let mut not_in_world_player = test_player_for_viewpoint(4930501);
    let not_in_world_player_guid = not_in_world_player.guid();
    not_in_world_player.set_farsight_object_like_cpp(guid(HighGuid::Creature, 4930502));
    not_in_world_player
        .unit_mut()
        .world_mut()
        .object_mut()
        .remove_from_world();
    not_in_world_map
        .insert_map_object_record(MapObjectRecord::new_player(not_in_world_player).unwrap())
        .unwrap();

    let not_in_world_removed = not_in_world_map
        .remove_from_map_like_cpp(not_in_world_player_guid, false)
        .unwrap();

    assert_eq!(not_in_world_removed.player_viewpoint_cleanup, None);

    let mut empty_map = test_map();
    let empty_player = test_player_for_viewpoint(4930601);
    let empty_player_guid = empty_player.guid();
    empty_map
        .insert_map_object_record(MapObjectRecord::new_player(empty_player).unwrap())
        .unwrap();

    let empty_removed = empty_map
        .remove_from_map_like_cpp(empty_player_guid, false)
        .unwrap();

    assert_eq!(empty_removed.player_viewpoint_cleanup, None);
}
#[test]
fn remove_list_drain_runs_switch_list_before_physical_remove_like_cpp() {
    let mut map = test_map();
    let spawn_id = 420080;
    let (guid, _, _) = add_loaded_grid_creature_for_switch(&mut map, spawn_id, 4200801);

    assert_eq!(
        map.add_object_to_switch_list_like_cpp(guid, true).status,
        AddObjectToSwitchListStatusLikeCpp::Queued
    );
    assert!(map.add_object_to_remove_list_like_cpp(guid).queued);
    let drain = map.remove_all_objects_in_remove_list_like_cpp();

    assert_eq!(drain.switch_processed, 1);
    assert_eq!(drain.switch_executed, 1);
    assert_eq!(drain.processed, 1);
    assert_eq!(drain.removed, 1);
    assert_eq!(map.objects_to_switch_count_like_cpp(), 0);
    assert_eq!(map.objects_to_remove_count_like_cpp(), 0);
    assert!(map.map_object_record(guid).is_none());
    assert_eq!(map.creature_spawn_id_store_count_like_cpp(spawn_id), 0);
}
#[test]
fn linked_trap_remove_owner_removes_trap_map_local_and_leaves_unrelated_objects() {
    let mut map = test_map();
    let mut owner = game_object_with_counter(10, 571, 7, false);
    let trap = game_object_with_counter(11, 571, 7, false);
    let unrelated = game_object_with_counter(12, 571, 7, false);
    let owner_guid = owner.world().guid();
    let trap_guid = trap.world().guid();
    let unrelated_guid = unrelated.world().guid();
    owner.set_linked_trap_like_cpp(trap_guid);

    map.add_map_object_record_to_map_like_cpp(MapObjectRecord::new_game_object(trap).unwrap())
        .unwrap();
    map.add_map_object_record_to_map_like_cpp(MapObjectRecord::new_game_object(unrelated).unwrap())
        .unwrap();
    map.add_map_object_record_to_map_like_cpp(MapObjectRecord::new_game_object(owner).unwrap())
        .unwrap();

    let removed = map.remove_from_map_like_cpp(owner_guid, true).unwrap();

    assert_eq!(removed.guid, owner_guid);
    assert!(map.map_object_record(owner_guid).is_none());
    assert!(map.map_object_record(trap_guid).is_some());
    assert_eq!(map.objects_to_remove_count_like_cpp(), 1);
    assert!(map.map_object_record(unrelated_guid).is_some());
}
#[test]
fn remove_from_map_like_cpp_can_delete_object_and_reports_missing_guid() {
    let mut map = test_map();
    let creature = world_object(HighGuid::Creature, 571, 7, false);
    let guid = creature.guid();
    map.add_to_map_like_cpp(AccessorObjectKind::Creature, creature)
        .unwrap();

    let removed = map.remove_from_map_like_cpp(guid, true).unwrap();
    assert!(removed.delete_from_world);
    assert!(removed.object.is_none());
    assert_eq!(map.map_object_count(), 0);

    assert_eq!(
        map.remove_from_map_like_cpp(guid, false),
        Err(RemoveFromMapError::ObjectNotFound { guid })
    );
}
#[test]
fn process_map_object_move_list_like_cpp_relocates_active_entries_and_resets_inactive() {
    let mut map = test_map();
    let creature = world_object_with_counter(HighGuid::Creature, 1, 571, 7, false);
    let creature_guid = creature.guid();
    let gameobject = world_object_with_counter(HighGuid::GameObject, 2, 571, 7, false);
    let gameobject_guid = gameobject.guid();
    map.add_to_map_like_cpp(AccessorObjectKind::Creature, creature)
        .unwrap();
    map.add_to_map_like_cpp(AccessorObjectKind::GameObject, gameobject)
        .unwrap();

    let plan = map.process_map_object_move_list_like_cpp([
        MapObjectMoveListEntry {
            guid: creature_guid,
            kind: AccessorObjectKind::Creature,
            move_state: MapObjectCellMoveState::Active,
            new_position: Position::xyz(5.0, 5.0, 3.0),
            respawn_position: None,
            is_pet: false,
        },
        MapObjectMoveListEntry {
            guid: gameobject_guid,
            kind: AccessorObjectKind::GameObject,
            move_state: MapObjectCellMoveState::Inactive,
            new_position: Position::xyz(6.0, 6.0, 3.0),
            respawn_position: None,
            is_pet: false,
        },
    ]);

    assert_eq!(plan.relocated, vec![creature_guid]);
    assert_eq!(plan.reset_inactive_or_none, vec![gameobject_guid]);
    assert_eq!(
        map.get_creature(creature_guid).unwrap().position(),
        Position::xyz(5.0, 5.0, 3.0)
    );
}
#[test]
fn process_map_object_move_list_like_cpp_uses_respawn_or_removal_fallbacks() {
    let mut map = test_map();
    let creature = world_object_with_counter(HighGuid::Creature, 1, 571, 7, false);
    let creature_guid = creature.guid();
    let gameobject = world_object_with_counter(HighGuid::GameObject, 2, 571, 7, false);
    let gameobject_guid = gameobject.guid();
    let pet = world_object_with_counter(HighGuid::Creature, 3, 571, 7, false);
    let pet_guid = pet.guid();
    map.add_to_map_like_cpp(AccessorObjectKind::Creature, creature)
        .unwrap();
    map.add_to_map_like_cpp(AccessorObjectKind::GameObject, gameobject)
        .unwrap();
    map.add_to_map_like_cpp(AccessorObjectKind::Creature, pet)
        .unwrap();

    let plan = map.process_map_object_move_list_like_cpp([
        MapObjectMoveListEntry {
            guid: creature_guid,
            kind: AccessorObjectKind::Creature,
            move_state: MapObjectCellMoveState::Active,
            new_position: Position::xyz(700.0, 20.0, 3.0),
            respawn_position: Some(Position::xyz(2.0, 2.0, 3.0)),
            is_pet: false,
        },
        MapObjectMoveListEntry {
            guid: gameobject_guid,
            kind: AccessorObjectKind::GameObject,
            move_state: MapObjectCellMoveState::Active,
            new_position: Position::xyz(700.0, 20.0, 3.0),
            respawn_position: None,
            is_pet: false,
        },
        MapObjectMoveListEntry {
            guid: pet_guid,
            kind: AccessorObjectKind::Creature,
            move_state: MapObjectCellMoveState::Active,
            new_position: Position::xyz(700.0, 20.0, 3.0),
            respawn_position: None,
            is_pet: true,
        },
    ]);

    assert_eq!(plan.respawn_relocated, vec![creature_guid]);
    assert_eq!(plan.remove_from_world, vec![gameobject_guid]);
    assert_eq!(plan.pet_removed, vec![pet_guid]);
    assert_eq!(
        map.get_creature(creature_guid).unwrap().position(),
        Position::xyz(2.0, 2.0, 3.0)
    );
}
#[test]
fn process_map_object_move_list_like_cpp_blocks_dynamic_and_skips_not_in_world() {
    let mut map = test_map();
    let dynamic = world_object_with_counter(HighGuid::DynamicObject, 1, 571, 7, false);
    let dynamic_guid = dynamic.guid();
    let area_trigger = world_object_with_counter(HighGuid::AreaTrigger, 2, 571, 7, false);
    let area_trigger_guid = area_trigger.guid();
    let offline_creature = world_object_with_counter(HighGuid::Creature, 3, 571, 7, false);
    let offline_creature_guid = offline_creature.guid();
    map.add_to_map_like_cpp(AccessorObjectKind::DynamicObject, dynamic)
        .unwrap();
    map.add_to_map_like_cpp(AccessorObjectKind::AreaTrigger, area_trigger)
        .unwrap();
    map.insert_map_object(AccessorObjectKind::Creature, offline_creature)
        .unwrap();

    let plan = map.process_map_object_move_list_like_cpp([
        MapObjectMoveListEntry {
            guid: dynamic_guid,
            kind: AccessorObjectKind::DynamicObject,
            move_state: MapObjectCellMoveState::Active,
            new_position: Position::xyz(700.0, 20.0, 3.0),
            respawn_position: None,
            is_pet: false,
        },
        MapObjectMoveListEntry {
            guid: area_trigger_guid,
            kind: AccessorObjectKind::AreaTrigger,
            move_state: MapObjectCellMoveState::Active,
            new_position: Position::xyz(700.0, 20.0, 3.0),
            respawn_position: None,
            is_pet: false,
        },
        MapObjectMoveListEntry {
            guid: offline_creature_guid,
            kind: AccessorObjectKind::Creature,
            move_state: MapObjectCellMoveState::Active,
            new_position: Position::xyz(2.0, 2.0, 3.0),
            respawn_position: None,
            is_pet: false,
        },
    ]);

    assert_eq!(
        plan.blocked_unloaded_grid,
        vec![dynamic_guid, area_trigger_guid]
    );
    assert_eq!(plan.skipped_not_in_world, vec![offline_creature_guid]);
}
