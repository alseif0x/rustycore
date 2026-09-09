//! Quest scenarios for [`super`].
//!
//! Split out of map_tests.rs under #628; assertions and
//! registrations are unchanged and shared fixtures stay in the parent module.

use super::*;

#[test]
fn unit_shared_vision_set_world_object_request_enqueues_like_cpp() {
    let mut map = test_map();
    let spawn_id = 423010;
    let (guid, cell, grid) = add_loaded_grid_creature_for_switch(&mut map, spawn_id, 4230101);

    let outcome = map.apply_unit_shared_vision_set_world_object_request_like_cpp(
        UnitSharedVisionSetWorldObjectRequestLikeCpp {
            unit_guid: guid,
            on: true,
        },
    );

    assert_eq!(outcome.guid, guid);
    assert_eq!(outcome.on, true);
    assert_eq!(
        outcome.status,
        SetWorldObjectStatusLikeCpp::Delegated(AddObjectToSwitchListStatusLikeCpp::Queued)
    );
    assert_eq!(map.objects_to_switch_count_like_cpp(), 1);
    assert_eq!(map.pending_switch_like_cpp(guid), Some(true));
    assert!(!map.get_typed_creature(guid).unwrap().is_temp_world_object());

    let drain = map.remove_all_objects_in_remove_list_like_cpp();

    assert_eq!(drain.switch_processed, 1);
    assert_eq!(drain.switch_executed, 1);
    assert!(map.map_object_record(guid).is_some());
    assert_eq!(map.creature_spawn_id_store_count_like_cpp(spawn_id), 1);
    let local_cell = local_cell_for_switch(&map, grid, cell);
    assert!(!local_cell.grid_objects.creatures.contains(&guid));
    assert!(local_cell.world_objects.creatures.contains(&guid));
    assert!(map.get_typed_creature(guid).unwrap().is_temp_world_object());
}
#[test]
fn unit_shared_vision_set_world_object_request_opposite_toggle_cancels_like_cpp() {
    let mut map = test_map();
    let (guid, cell, grid) = add_loaded_grid_creature_for_switch(&mut map, 423020, 4230201);

    assert_eq!(
        map.apply_unit_shared_vision_set_world_object_request_like_cpp(
            UnitSharedVisionSetWorldObjectRequestLikeCpp {
                unit_guid: guid,
                on: true,
            },
        )
        .status,
        SetWorldObjectStatusLikeCpp::Delegated(AddObjectToSwitchListStatusLikeCpp::Queued)
    );
    assert_eq!(
        map.apply_unit_shared_vision_set_world_object_request_like_cpp(
            UnitSharedVisionSetWorldObjectRequestLikeCpp {
                unit_guid: guid,
                on: false,
            },
        )
        .status,
        SetWorldObjectStatusLikeCpp::Delegated(
            AddObjectToSwitchListStatusLikeCpp::CancelledOppositeToggle
        )
    );
    assert_eq!(map.objects_to_switch_count_like_cpp(), 0);

    let drain = map.remove_all_objects_in_remove_list_like_cpp();

    assert_eq!(drain.switch_processed, 0);
    assert_eq!(drain.switch_executed, 0);
    let local_cell = local_cell_for_switch(&map, grid, cell);
    assert!(local_cell.grid_objects.creatures.contains(&guid));
    assert!(!local_cell.world_objects.creatures.contains(&guid));
    assert!(!map.get_typed_creature(guid).unwrap().is_temp_world_object());
}
#[test]
fn unit_shared_vision_set_world_object_request_uses_existing_fallbacks_like_cpp() {
    let mut map = test_map();
    let missing_guid = guid(HighGuid::Creature, 4230301);

    let missing = map.apply_unit_shared_vision_set_world_object_request_like_cpp(
        UnitSharedVisionSetWorldObjectRequestLikeCpp {
            unit_guid: missing_guid,
            on: true,
        },
    );

    assert_eq!(missing.status, SetWorldObjectStatusLikeCpp::MissingOrStale);
    assert_eq!(map.objects_to_switch_count_like_cpp(), 0);
    assert_eq!(map.map_object_count(), 0);

    let gameobject = test_gameobject_for_spawn(423030, 4230302);
    let gameobject_guid = gameobject.world().guid();
    map.insert_map_object_record(MapObjectRecord::new_game_object(gameobject).unwrap())
        .unwrap();

    let non_unit = map.apply_unit_shared_vision_set_world_object_request_like_cpp(
        UnitSharedVisionSetWorldObjectRequestLikeCpp {
            unit_guid: gameobject_guid,
            on: true,
        },
    );

    assert_eq!(
        non_unit.status,
        SetWorldObjectStatusLikeCpp::Delegated(AddObjectToSwitchListStatusLikeCpp::IgnoredNonUnit)
    );
    assert_eq!(map.objects_to_switch_count_like_cpp(), 0);
    assert_eq!(map.map_object_count(), 1);
    let drain = map.remove_all_objects_in_remove_list_like_cpp();
    assert_eq!(drain.switch_processed, 0);
    assert_eq!(map.map_object_count(), 1);
}
