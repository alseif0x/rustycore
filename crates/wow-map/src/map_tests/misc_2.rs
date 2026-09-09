//! Misc scenarios for [`super`].
//!
//! Split out of map_tests.rs under #628; assertions and
//! registrations are unchanged and shared fixtures stay in the parent module.

use super::*;

#[test]
fn transport_update_wrong_kind_missing_untyped_skip_but_not_in_world_updates_like_cpp() {
    let mut map = test_map();
    let missing_guid = guid(HighGuid::Transport, 4390201);
    let creature = test_creature_for_spawn(43902, 4390202, true);
    let creature_guid = creature.guid();
    map.insert_map_object_record(MapObjectRecord::new_creature(creature).unwrap())
        .unwrap();
    let not_in_world = test_transport_for_update(4390203, false);
    let not_in_world_guid = not_in_world.world().guid();
    map.insert_map_object_record(MapObjectRecord::new_transport(not_in_world).unwrap())
        .unwrap();
    let untyped_transport = world_object_with_counter(HighGuid::Transport, 4390204, 571, 7, true);
    let untyped_guid = untyped_transport.guid();
    map.insert_map_object(AccessorObjectKind::Transport, untyped_transport)
        .unwrap();

    let missing = map.update_transport_like_cpp(missing_guid, 50, 10_000);
    let wrong_kind = map.update_transport_like_cpp(creature_guid, 50, 10_000);
    let not_in_world = map.update_transport_like_cpp(not_in_world_guid, 50, 10_000);
    let untyped = map.update_transport_like_cpp(untyped_guid, 50, 10_000);

    assert_eq!(
        missing.status,
        TransportUpdateStatusLikeCpp::MissingTransport
    );
    assert_eq!(
        wrong_kind.status,
        TransportUpdateStatusLikeCpp::NotTransport
    );
    assert_eq!(not_in_world.status, TransportUpdateStatusLikeCpp::Updated);
    assert_eq!(not_in_world.path_progress_before_ms, Some(100));
    assert_eq!(not_in_world.path_progress_after_ms, Some(150));
    assert_eq!(untyped.status, TransportUpdateStatusLikeCpp::NotTransport);
    assert_eq!(map.map_object_count(), 3);
    let transport = map
        .map_object_record(not_in_world_guid)
        .and_then(MapObjectRecord::transport)
        .unwrap();
    assert_eq!(transport.path_progress_ms(), 150);
}
#[test]
fn transports_update_summary_snapshots_only_typed_transports_like_cpp() {
    let mut map = test_map();
    let typed_transport = test_transport_for_update(4390301, true);
    let typed_guid = typed_transport.world().guid();
    map.insert_map_object_record(MapObjectRecord::new_transport(typed_transport).unwrap())
        .unwrap();
    let untyped_transport = world_object_with_counter(HighGuid::Transport, 4390302, 571, 7, true);
    map.insert_map_object(AccessorObjectKind::Transport, untyped_transport)
        .unwrap();
    let creature = test_creature_for_spawn(43903, 4390303, true);
    map.insert_map_object_record(MapObjectRecord::new_creature(creature).unwrap())
        .unwrap();

    let summary = map.update_transports_like_cpp(250, 10_000);

    assert_eq!(
        summary,
        TransportsUpdateSummaryLikeCpp {
            visited: 1,
            updated: 1,
            unsupported_no_period: 0,
            missing_or_stale: 0,
            not_transport: 0,
            not_in_world: 0,
            position_updates_represented: 1,
            just_stopped: 0,
        }
    );
    let transport = map
        .map_object_record(typed_guid)
        .and_then(MapObjectRecord::transport)
        .unwrap();
    assert_eq!(transport.path_progress_ms(), 350);
}
#[test]
fn transport_update_period_zero_reports_unsupported_without_mutation_like_cpp() {
    let mut map = test_map();
    let mut transport = test_transport_for_update(4390401, true);
    let transport_guid = transport.world().guid();
    transport.set_period(0);
    transport.set_path_progress_ms(333);
    map.insert_map_object_record(MapObjectRecord::new_transport(transport).unwrap())
        .unwrap();

    let outcome = map.update_transport_like_cpp(transport_guid, 50, 10_000);

    assert_eq!(
        outcome.status,
        TransportUpdateStatusLikeCpp::UnsupportedNoPeriod
    );
    assert_eq!(outcome.path_progress_before_ms, Some(333));
    assert_eq!(outcome.path_progress_after_ms, Some(333));
    assert_eq!(outcome.timer_ms, None);
    let transport = map
        .map_object_record(transport_guid)
        .and_then(MapObjectRecord::transport)
        .unwrap();
    assert_eq!(transport.path_progress_ms(), 333);
}
#[test]
fn scene_object_update_not_in_world_returns_no_mutation_or_queue_like_cpp() {
    let mut map = test_map();
    let scene_object = test_scene_object_for_update(4370401, false, guid(HighGuid::Cast, 4370402));
    let scene_object_guid = scene_object.world().guid();
    map.insert_map_object_record(MapObjectRecord::new_scene_object(scene_object).unwrap())
        .unwrap();

    let outcome = map.update_scene_object_like_cpp(
        scene_object_guid,
        250,
        SceneObjectUpdateContextLikeCpp {
            creator_exists: false,
            linked_aura_exists: false,
        },
    );

    assert_eq!(outcome.status, SceneObjectUpdateStatusLikeCpp::NotInWorld);
    assert!(!outcome.world_update_would_run);
    assert!(!outcome.should_be_removed);
    assert!(outcome.remove_list.is_none());
    assert_eq!(map.objects_to_remove_count_like_cpp(), 0);
    assert!(map.map_object_record(scene_object_guid).is_some());
}
#[test]
fn scene_object_update_missing_non_scene_or_untyped_creates_no_dummy_or_queue_like_cpp() {
    let mut map = test_map();
    let missing_guid = guid(HighGuid::SceneObject, 4370501);
    let creature = test_creature_for_spawn(43705, 4370502, true);
    let creature_guid = creature.guid();
    map.insert_map_object_record(MapObjectRecord::new_creature(creature).unwrap())
        .unwrap();
    let untyped_scene = world_object_with_counter(HighGuid::SceneObject, 4370503, 571, 7, true);
    let untyped_scene_guid = untyped_scene.guid();
    map.insert_map_object(AccessorObjectKind::SceneObject, untyped_scene)
        .unwrap();

    let context = SceneObjectUpdateContextLikeCpp {
        creator_exists: false,
        linked_aura_exists: false,
    };
    let missing = map.update_scene_object_like_cpp(missing_guid, 250, context);
    let non_scene = map.update_scene_object_like_cpp(creature_guid, 250, context);
    let untyped = map.update_scene_object_like_cpp(untyped_scene_guid, 250, context);

    assert_eq!(
        missing.status,
        SceneObjectUpdateStatusLikeCpp::MissingSceneObject
    );
    assert_eq!(
        non_scene.status,
        SceneObjectUpdateStatusLikeCpp::NotSceneObject
    );
    assert_eq!(
        untyped.status,
        SceneObjectUpdateStatusLikeCpp::NotSceneObject
    );
    assert_eq!(missing.remove_list, None);
    assert_eq!(non_scene.remove_list, None);
    assert_eq!(untyped.remove_list, None);
    assert_eq!(map.objects_to_remove_count_like_cpp(), 0);
    assert_eq!(map.map_object_count(), 2);
    assert!(map.map_object_record(missing_guid).is_none());
    assert!(map.map_object_record(creature_guid).is_some());
    assert!(map.map_object_record(untyped_scene_guid).is_some());
}
#[test]
fn scene_objects_update_summary_snapshots_only_typed_scene_objects_like_cpp() {
    let mut map = test_map();
    let typed_scene = test_scene_object_for_update(4370601, true, ObjectGuid::EMPTY);
    let typed_scene_guid = typed_scene.world().guid();
    map.insert_map_object_record(MapObjectRecord::new_scene_object(typed_scene).unwrap())
        .unwrap();
    let untyped_scene = world_object_with_counter(HighGuid::SceneObject, 4370602, 571, 7, true);
    map.insert_map_object(AccessorObjectKind::SceneObject, untyped_scene)
        .unwrap();
    let creature = test_creature_for_spawn(43706, 4370603, true);
    map.insert_map_object_record(MapObjectRecord::new_creature(creature).unwrap())
        .unwrap();

    let summary = map.update_scene_objects_like_cpp(250, |_guid, scene_object| {
        assert_eq!(scene_object.world().guid(), typed_scene_guid);
        SceneObjectUpdateContextLikeCpp {
            creator_exists: true,
            linked_aura_exists: true,
        }
    });

    assert_eq!(
        summary,
        SceneObjectsUpdateSummaryLikeCpp {
            visited: 1,
            updated: 1,
            remove_queued: 0,
            missing_or_stale: 0,
            not_scene_object: 0,
            not_in_world: 0,
        }
    );
    assert_eq!(map.objects_to_remove_count_like_cpp(), 0);
}
#[test]
fn conversation_update_decrements_duration_without_queue_like_cpp() {
    let mut map = test_map();
    let conversation = test_conversation_for_update(4360101, 1_000, true);
    let conversation_guid = conversation.world().guid();
    map.insert_map_object_record(MapObjectRecord::new_conversation(conversation).unwrap())
        .unwrap();

    let outcome = map.update_conversation_like_cpp(conversation_guid, 250);

    assert_eq!(outcome.status, ConversationUpdateStatusLikeCpp::Updated);
    assert_eq!(outcome.duration_before_ms, Some(1_000));
    assert_eq!(outcome.duration_after_ms, Some(750));
    assert!(outcome.script_update_would_run);
    assert!(outcome.world_update_would_run);
    assert!(outcome.remove_list.is_none());
    assert_eq!(map.objects_to_remove_count_like_cpp(), 0);
    let conversation = map
        .map_object_record(conversation_guid)
        .unwrap()
        .conversation()
        .unwrap();
    assert_eq!(conversation.duration_ms(), 750);
    assert!(!conversation.is_removed());
}
#[test]
fn conversation_update_not_in_world_returns_no_mutation_or_queue_like_cpp() {
    let mut map = test_map();
    let conversation = test_conversation_for_update(4360301, 500, false);
    let conversation_guid = conversation.world().guid();
    map.insert_map_object_record(MapObjectRecord::new_conversation(conversation).unwrap())
        .unwrap();

    let outcome = map.update_conversation_like_cpp(conversation_guid, 250);

    assert_eq!(outcome.status, ConversationUpdateStatusLikeCpp::NotInWorld);
    assert_eq!(outcome.duration_before_ms, Some(500));
    assert_eq!(outcome.duration_after_ms, Some(500));
    assert!(!outcome.script_update_would_run);
    assert!(!outcome.world_update_would_run);
    assert!(outcome.remove_list.is_none());
    assert_eq!(map.objects_to_remove_count_like_cpp(), 0);
    let conversation = map
        .map_object_record(conversation_guid)
        .unwrap()
        .conversation()
        .unwrap();
    assert_eq!(conversation.duration_ms(), 500);
    assert!(!conversation.is_removed());
}
#[test]
fn conversation_update_missing_non_conversation_or_untyped_creates_no_dummy_or_queue_like_cpp() {
    let mut map = test_map();
    let missing_guid = guid(HighGuid::Conversation, 4360401);
    let creature = test_creature_for_spawn(43604, 4360402, true);
    let creature_guid = creature.guid();
    map.insert_map_object_record(MapObjectRecord::new_creature(creature).unwrap())
        .unwrap();
    let untyped_conversation =
        world_object_with_counter(HighGuid::Conversation, 4360403, 571, 7, true);
    let untyped_conversation_guid = untyped_conversation.guid();
    map.insert_map_object(AccessorObjectKind::Conversation, untyped_conversation)
        .unwrap();

    let missing = map.update_conversation_like_cpp(missing_guid, 250);
    let non_conversation = map.update_conversation_like_cpp(creature_guid, 250);
    let untyped = map.update_conversation_like_cpp(untyped_conversation_guid, 250);

    assert_eq!(
        missing.status,
        ConversationUpdateStatusLikeCpp::MissingConversation
    );
    assert_eq!(
        non_conversation.status,
        ConversationUpdateStatusLikeCpp::NotConversation
    );
    assert_eq!(
        untyped.status,
        ConversationUpdateStatusLikeCpp::NotConversation
    );
    assert_eq!(missing.remove_list, None);
    assert_eq!(non_conversation.remove_list, None);
    assert_eq!(untyped.remove_list, None);
    assert_eq!(map.objects_to_remove_count_like_cpp(), 0);
    assert_eq!(map.map_object_count(), 2);
    assert!(map.map_object_record(missing_guid).is_none());
    assert!(map.map_object_record(creature_guid).is_some());
    assert!(map.map_object_record(untyped_conversation_guid).is_some());
}
#[test]
fn dynamic_object_update_not_in_world_returns_no_mutation_or_queue_like_cpp() {
    let mut map = test_map();
    let mut dynamic_object = test_dynamic_object_for_viewpoint(4290401);
    let dynamic_object_guid = dynamic_object.world().guid();
    dynamic_object.set_duration(1_000);
    dynamic_object.world_mut().object_mut().remove_from_world();
    map.insert_map_object_record(MapObjectRecord::new_dynamic_object(dynamic_object).unwrap())
        .unwrap();

    let outcome = map.update_dynamic_object_like_cpp(dynamic_object_guid, 250);

    assert_eq!(outcome.status, DynamicObjectUpdateStatusLikeCpp::NotInWorld);
    assert_eq!(outcome.duration_before_ms, Some(1_000));
    assert_eq!(outcome.duration_after_ms, Some(1_000));
    assert!(!outcome.script_update_would_run);
    assert_eq!(outcome.remove_list, None);
    assert_eq!(map.objects_to_remove_count_like_cpp(), 0);
    let dynamic_object = map.get_typed_dynamic_object(dynamic_object_guid).unwrap();
    assert_eq!(dynamic_object.duration_ms(), 1_000);
    assert!(!dynamic_object.world().object().is_destroyed_object());
}
#[test]
fn dynamic_object_update_missing_or_non_dynamic_creates_no_dummy_or_queue_like_cpp() {
    let mut map = test_map();
    let missing_guid = guid(HighGuid::DynamicObject, 4290601);
    let creature = world_object_with_counter(HighGuid::Creature, 4290602, 571, 7, true);
    let creature_guid = creature.guid();
    map.insert_map_object(AccessorObjectKind::Creature, creature)
        .unwrap();
    let untyped_dynamic = world_object_with_counter(HighGuid::DynamicObject, 4290603, 571, 7, true);
    let untyped_dynamic_guid = untyped_dynamic.guid();
    map.insert_map_object(AccessorObjectKind::DynamicObject, untyped_dynamic)
        .unwrap();

    let missing = map.update_dynamic_object_like_cpp(missing_guid, 250);
    let non_dynamic = map.update_dynamic_object_like_cpp(creature_guid, 250);
    let untyped = map.update_dynamic_object_like_cpp(untyped_dynamic_guid, 250);

    assert_eq!(
        missing.status,
        DynamicObjectUpdateStatusLikeCpp::MissingDynamicObject
    );
    assert_eq!(
        non_dynamic.status,
        DynamicObjectUpdateStatusLikeCpp::NotDynamicObject
    );
    assert_eq!(
        untyped.status,
        DynamicObjectUpdateStatusLikeCpp::NotDynamicObject
    );
    assert_eq!(missing.remove_list, None);
    assert_eq!(non_dynamic.remove_list, None);
    assert_eq!(untyped.remove_list, None);
    assert_eq!(map.objects_to_remove_count_like_cpp(), 0);
    assert_eq!(map.map_object_count(), 2);
    assert!(map.map_object_record(missing_guid).is_none());
    assert!(map.map_object_record(creature_guid).is_some());
    assert!(map.map_object_record(untyped_dynamic_guid).is_some());
}
#[test]
fn player_set_viewpoint_apply_unit_target_consumes_set_world_object_like_cpp() {
    let mut map = test_map();
    let player = test_player_for_viewpoint(4240101);
    let player_guid = player.guid();
    let (target_guid, _cell, _grid) =
        add_loaded_grid_creature_for_switch(&mut map, 424010, 4240102);
    map.insert_map_object_record(MapObjectRecord::new_player(player).unwrap())
        .unwrap();

    let outcome =
        map.apply_player_set_viewpoint_unit_like_cpp(player_guid, target_guid, true, None);

    assert_eq!(outcome.status, PlayerSetViewpointStatusLikeCpp::Applied);
    assert!(outcome.update_visibility_requested);
    assert!(outcome.set_seer_requested);
    assert_eq!(
        outcome.set_world_object,
        Some(SetWorldObjectOutcomeLikeCpp {
            guid: target_guid,
            on: true,
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
        target_guid
    );
    assert!(
        map.get_typed_creature(target_guid)
            .unwrap()
            .unit()
            .subsystems()
            .control
            .shared_vision_guids
            .contains(&player_guid)
    );
    assert_eq!(map.pending_switch_like_cpp(target_guid), Some(true));
}
#[test]
fn player_set_viewpoint_apply_existing_viewpoint_is_no_mutation_like_cpp() {
    let mut map = test_map();
    let mut player = test_player_for_viewpoint(4240201);
    let player_guid = player.guid();
    let existing_guid = guid(HighGuid::Creature, 4240209);
    player.set_farsight_object_like_cpp(existing_guid);
    let (target_guid, _cell, _grid) =
        add_loaded_grid_creature_for_switch(&mut map, 424020, 4240202);
    map.insert_map_object_record(MapObjectRecord::new_player(player).unwrap())
        .unwrap();

    let outcome =
        map.apply_player_set_viewpoint_unit_like_cpp(player_guid, target_guid, true, None);

    assert_eq!(
        outcome.status,
        PlayerSetViewpointStatusLikeCpp::AlreadyHasViewpoint
    );
    assert_eq!(outcome.set_world_object, None);
    assert!(!outcome.update_visibility_requested);
    assert!(!outcome.set_seer_requested);
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
fn player_set_viewpoint_vehicle_base_skips_unit_shared_vision_like_cpp() {
    let mut map = test_map();
    let player = test_player_for_viewpoint(4240501);
    let player_guid = player.guid();
    let (target_guid, _cell, _grid) =
        add_loaded_grid_creature_for_switch(&mut map, 424050, 4240502);
    map.insert_map_object_record(MapObjectRecord::new_player(player).unwrap())
        .unwrap();

    let outcome = map.apply_player_set_viewpoint_unit_like_cpp(
        player_guid,
        target_guid,
        true,
        Some(target_guid),
    );

    assert_eq!(outcome.status, PlayerSetViewpointStatusLikeCpp::Applied);
    assert!(outcome.update_visibility_requested);
    assert!(outcome.set_seer_requested);
    assert_eq!(outcome.set_world_object, None);
    assert_eq!(
        map.get_typed_player(player_guid)
            .unwrap()
            .active_data()
            .farsight_object,
        target_guid
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
fn set_world_object_like_cpp_non_unit_in_world_uses_ignored_outcome_without_queue() {
    let mut map = test_map();
    let gameobject = test_gameobject_for_spawn(421030, 4210301);
    let guid = gameobject.world().guid();
    map.insert_map_object_record(MapObjectRecord::new_game_object(gameobject).unwrap())
        .unwrap();

    let outcome = map.set_world_object_like_cpp(guid, true);

    assert_eq!(
        outcome.status,
        SetWorldObjectStatusLikeCpp::Delegated(AddObjectToSwitchListStatusLikeCpp::IgnoredNonUnit)
    );
    assert_eq!(map.objects_to_switch_count_like_cpp(), 0);
    assert_eq!(map.map_object_count(), 1);
}
#[test]
fn set_world_object_like_cpp_missing_stale_does_not_create_records() {
    let mut map = test_map();
    let guid = guid(HighGuid::Creature, 4210401);

    let outcome = map.set_world_object_like_cpp(guid, true);

    assert_eq!(outcome.status, SetWorldObjectStatusLikeCpp::MissingOrStale);
    assert_eq!(map.objects_to_switch_count_like_cpp(), 0);
    assert_eq!(map.map_object_count(), 0);
    let drain = map.remove_all_objects_in_remove_list_like_cpp();
    assert_eq!(drain.switch_processed, 0);
    assert_eq!(map.map_object_count(), 0);
}
#[test]
fn set_world_object_like_cpp_opposite_toggle_cancels_before_drain() {
    let mut map = test_map();
    let (guid, cell, grid) = add_loaded_grid_creature_for_switch(&mut map, 421050, 4210501);

    assert_eq!(
        map.set_world_object_like_cpp(guid, true).status,
        SetWorldObjectStatusLikeCpp::Delegated(AddObjectToSwitchListStatusLikeCpp::Queued)
    );
    assert!(!map.get_typed_creature(guid).unwrap().is_temp_world_object());
    assert_eq!(
        map.set_world_object_like_cpp(guid, false).status,
        SetWorldObjectStatusLikeCpp::Delegated(
            AddObjectToSwitchListStatusLikeCpp::CancelledOppositeToggle
        )
    );
    assert_eq!(map.objects_to_switch_count_like_cpp(), 0);

    let drain = map.remove_all_objects_in_remove_list_like_cpp();

    assert_eq!(drain.switch_processed, 0);
    let local_cell = local_cell_for_switch(&map, grid, cell);
    assert!(local_cell.grid_objects.creatures.contains(&guid));
    assert!(!local_cell.world_objects.creatures.contains(&guid));
    assert!(!map.get_typed_creature(guid).unwrap().is_temp_world_object());
}
#[test]
fn switch_list_opposite_toggle_before_drain_cancels_like_cpp() {
    let mut map = test_map();
    let (guid, cell, grid) = add_loaded_grid_creature_for_switch(&mut map, 420030, 4200301);

    assert_eq!(
        map.add_object_to_switch_list_like_cpp(guid, true).status,
        AddObjectToSwitchListStatusLikeCpp::Queued
    );
    assert_eq!(
        map.add_object_to_switch_list_like_cpp(guid, false).status,
        AddObjectToSwitchListStatusLikeCpp::CancelledOppositeToggle
    );
    assert_eq!(map.objects_to_switch_count_like_cpp(), 0);
    let drain = map.remove_all_objects_in_remove_list_like_cpp();

    assert_eq!(drain.switch_processed, 0);
    let local_cell = local_cell_for_switch(&map, grid, cell);
    assert!(local_cell.grid_objects.creatures.contains(&guid));
    assert!(!local_cell.world_objects.creatures.contains(&guid));
    assert!(!map.get_typed_creature(guid).unwrap().is_temp_world_object());
}
#[test]
fn switch_list_duplicate_same_direction_reports_abort_outcome_like_cpp() {
    let mut map = test_map();
    let (guid, _, _) = add_loaded_grid_creature_for_switch(&mut map, 420040, 4200401);

    assert_eq!(
        map.add_object_to_switch_list_like_cpp(guid, true).status,
        AddObjectToSwitchListStatusLikeCpp::Queued
    );
    assert_eq!(
        map.add_object_to_switch_list_like_cpp(guid, true).status,
        AddObjectToSwitchListStatusLikeCpp::DuplicateSameDirectionAbort
    );
    assert_eq!(map.objects_to_switch_count_like_cpp(), 1);
    assert_eq!(map.pending_switch_like_cpp(guid), Some(true));
}
#[test]
fn switch_list_stale_guid_drain_does_not_create_dummy_like_cpp() {
    let mut map = test_map();
    let guid = guid(HighGuid::Creature, 4200601);
    map.enqueue_object_to_switch_for_test(guid, true);

    let drain = map.remove_all_objects_in_remove_list_like_cpp();

    assert_eq!(drain.switch_processed, 1);
    assert_eq!(drain.switch_missing_or_stale, 1);
    assert_eq!(map.objects_to_switch_count_like_cpp(), 0);
    assert_eq!(map.map_object_count(), 0);
}
#[test]
fn process_relocation_notifies_like_cpp_selects_delayed_before_resetting_flags() {
    let mut map = test_map();
    let creature = world_object_with_counter(HighGuid::Creature, 1, 571, 7, false);
    let creature_guid = creature.guid();
    let player = world_object_with_counter(HighGuid::Player, 2, 571, 7, false);
    let player_guid = player.guid();
    let cell = map
        .add_to_map_like_cpp(AccessorObjectKind::Creature, creature)
        .unwrap()
        .cell;
    let active_cell = Cell::from_cell_coord(cell);
    let active_grid = GridCoord::new(active_cell.grid_x(), active_cell.grid_y());
    map.get_ngrid_mut(active_grid)
        .unwrap()
        .set_state(GridStateKind::Active);
    map.add_to_map_like_cpp(AccessorObjectKind::Player, player)
        .unwrap();
    for guid in [creature_guid, player_guid] {
        map.entity_world
            .get_mut(&guid)
            .unwrap()
            .object_mut()
            .object_mut()
            .add_to_notify(ObjectNotifyFlags::VISIBILITY_CHANGED);
    }

    let outcome = map.process_relocation_notifies_like_cpp(
        [cell],
        1000,
        1000,
        std::iter::empty::<ObjectGuid>(),
    );

    assert_eq!(outcome.process_plan.delayed_relocation_cells, vec![cell]);
    assert_eq!(outcome.process_plan.reset_notify_cells, vec![cell]);
    assert_eq!(outcome.process_plan.reset_timer_grids, vec![active_grid]);
    assert_eq!(outcome.delayed_plan.cell_plans.len(), 1);
    assert_eq!(
        outcome.delayed_plan.cell_plans[0].plan.creature_relocations,
        vec![creature_guid]
    );
    assert_eq!(
        outcome.delayed_plan.cell_plans[0].plan.player_relocations,
        vec![player_guid]
    );
    assert_eq!(outcome.reset_outcome.reset_player_guids, vec![player_guid]);
    assert_eq!(
        outcome.reset_outcome.reset_creature_guids,
        vec![creature_guid]
    );
    assert!(
        !map.map_object(creature_guid)
            .unwrap()
            .object()
            .is_need_notify(ObjectNotifyFlags::VISIBILITY_CHANGED)
    );
    assert!(
        !map.map_object(player_guid)
            .unwrap()
            .object()
            .is_need_notify(ObjectNotifyFlags::VISIBILITY_CHANGED)
    );
}
