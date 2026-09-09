//! Spell scenarios for [`super`].
//!
//! Split out of map_tests.rs under #628; assertions and
//! registrations are unchanged and shared fixtures stay in the parent module.

use super::*;

#[test]
fn remove_all_dynamic_objects_for_caster_removes_only_matching_caster_like_cpp() {
    let mut map = test_map();
    let caster_guid = guid(HighGuid::Player, 4290701);
    let other_caster_guid = guid(HighGuid::Player, 4290702);

    let mut matching_dynamic = test_dynamic_object_for_viewpoint(4290703);
    let matching_guid = matching_dynamic.world().guid();
    matching_dynamic.set_caster_guid(caster_guid);
    map.insert_map_object_record(MapObjectRecord::new_dynamic_object(matching_dynamic).unwrap())
        .unwrap();

    let mut other_dynamic = test_dynamic_object_for_viewpoint(4290704);
    let other_guid = other_dynamic.world().guid();
    other_dynamic.set_caster_guid(other_caster_guid);
    map.insert_map_object_record(MapObjectRecord::new_dynamic_object(other_dynamic).unwrap())
        .unwrap();

    let outcome = map.remove_all_dynamic_objects_for_caster_like_cpp(caster_guid);

    assert_eq!(outcome.caster_guid, caster_guid);
    assert_eq!(outcome.candidates, 1);
    assert_eq!(outcome.removed, 1);
    assert_eq!(outcome.missing_or_stale, 0);
    assert_eq!(outcome.remove_errors, 0);
    assert!(map.map_object_record(matching_guid).is_none());
    assert!(map.map_object_record(other_guid).is_some());
}
#[test]
fn remove_all_dynamic_objects_for_caster_uses_dynamic_object_cleanup_like_cpp() {
    let mut map = test_map();
    let player = test_player_for_viewpoint(4290711);
    let player_guid = player.guid();
    map.insert_map_object_record(MapObjectRecord::new_player(player).unwrap())
        .unwrap();
    let create = create_farsight_focus_for_tests(&mut map, player_guid);
    assert_eq!(
        create.status,
        FarsightDynamicObjectCreateStatusLikeCpp::Created
    );
    let dynamic_guid = create.dynamic_object_guid.unwrap();
    {
        let dynamic_object = map.get_typed_dynamic_object_mut(dynamic_guid).unwrap();
        dynamic_object.set_aura_bound();
        assert_eq!(dynamic_object.bound_caster(), Some(player_guid));
        assert!(dynamic_object.has_aura());
    }

    let outcome = map.remove_all_dynamic_objects_for_caster_like_cpp(player_guid);

    assert_eq!(outcome.candidates, 1);
    assert_eq!(outcome.removed, 1);
    assert_eq!(outcome.dynamic_object_remove_aura_cleanup_count, 1);
    assert_eq!(outcome.dynamic_object_unbound_caster_count, 1);
    assert!(map.map_object_record(dynamic_guid).is_none());
    assert_eq!(
        map.get_typed_player(player_guid)
            .unwrap()
            .active_data()
            .farsight_object,
        ObjectGuid::EMPTY
    );
}
#[test]
fn dynamic_object_update_aura_bound_not_expired_runs_represented_update_owner_like_cpp() {
    let mut map = test_map();
    let mut dynamic_object = test_dynamic_object_for_viewpoint(4290501);
    let dynamic_object_guid = dynamic_object.world().guid();
    dynamic_object.set_duration(1_000);
    dynamic_object.set_aura_bound();
    map.insert_map_object_record(MapObjectRecord::new_dynamic_object(dynamic_object).unwrap())
        .unwrap();

    let outcome = map.update_dynamic_object_like_cpp(dynamic_object_guid, 250);

    assert_eq!(outcome.status, DynamicObjectUpdateStatusLikeCpp::Updated);
    assert_eq!(outcome.duration_before_ms, Some(1_000));
    assert_eq!(outcome.duration_after_ms, Some(1_000));
    assert_eq!(outcome.aura_update_owner_calls_before, Some(0));
    assert_eq!(outcome.aura_update_owner_calls_after, Some(1));
    assert!(outcome.script_update_would_run);
    assert_eq!(outcome.remove_list, None);
    assert_eq!(map.objects_to_remove_count_like_cpp(), 0);
    let dynamic_object = map.get_typed_dynamic_object(dynamic_object_guid).unwrap();
    assert_eq!(dynamic_object.duration_ms(), 1_000);
    assert!(dynamic_object.has_aura());
    assert_eq!(dynamic_object.represented_aura_update_owner_count(), 1);
    assert!(!dynamic_object.world().object().is_destroyed_object());
}
#[test]
fn dynamic_object_update_aura_bound_expired_queues_remove_and_drain_cleans_aura_caster_like_cpp() {
    let mut map = test_map();
    let player = test_player_for_viewpoint(4310201);
    let player_guid = player.guid();
    map.insert_map_object_record(MapObjectRecord::new_player(player).unwrap())
        .unwrap();
    let create = create_farsight_focus_for_tests(&mut map, player_guid);
    assert_eq!(
        create.status,
        FarsightDynamicObjectCreateStatusLikeCpp::Created
    );
    let dynamic_object_guid = create.dynamic_object_guid.unwrap();
    {
        let dynamic_object = map
            .get_typed_dynamic_object_mut(dynamic_object_guid)
            .unwrap();
        dynamic_object.set_duration(1_000);
        dynamic_object.set_aura_bound();
        dynamic_object.set_aura_removed_like_cpp(true);
    }

    let outcome = map.update_dynamic_object_like_cpp(dynamic_object_guid, 250);

    assert_eq!(
        outcome.status,
        DynamicObjectUpdateStatusLikeCpp::ExpiredRemoveQueued
    );
    assert_eq!(outcome.duration_before_ms, Some(1_000));
    assert_eq!(outcome.duration_after_ms, Some(1_000));
    assert_eq!(outcome.aura_update_owner_calls_before, Some(0));
    assert_eq!(outcome.aura_update_owner_calls_after, Some(0));
    assert!(!outcome.script_update_would_run);
    assert!(outcome.remove_list.unwrap().queued);
    assert_eq!(map.objects_to_remove_count_like_cpp(), 1);
    let dynamic_object = map.get_typed_dynamic_object(dynamic_object_guid).unwrap();
    assert!(dynamic_object.has_aura());
    assert_eq!(dynamic_object.bound_caster(), Some(player_guid));
    assert!(dynamic_object.world().object().is_destroyed_object());

    let drain = map.remove_all_objects_in_remove_list_like_cpp();

    assert_eq!(drain.processed, 1);
    assert_eq!(drain.removed, 1);
    assert_eq!(drain.dynamic_object_remove_aura_cleanup_count, 1);
    assert_eq!(drain.dynamic_object_unbound_caster_count, 1);
    assert!(map.map_object_record(dynamic_object_guid).is_none());
}
#[test]
fn player_remove_from_world_viewpoint_dynamic_object_ignores_bound_caster_like_cpp() {
    let mut missing_caster_map = test_map();
    let mut missing_caster_player = test_player_for_viewpoint(4930111);
    let missing_caster_player_guid = missing_caster_player.guid();
    let mut missing_caster_dynamic_object = test_dynamic_object_for_viewpoint(4930112);
    let missing_caster_dynamic_object_guid = missing_caster_dynamic_object.world().guid();
    missing_caster_player.set_farsight_object_like_cpp(missing_caster_dynamic_object_guid);
    missing_caster_dynamic_object.set_caster_viewpoint();
    missing_caster_map
        .insert_map_object_record(MapObjectRecord::new_player(missing_caster_player).unwrap())
        .unwrap();
    missing_caster_map
        .insert_map_object_record(
            MapObjectRecord::new_dynamic_object(missing_caster_dynamic_object).unwrap(),
        )
        .unwrap();

    let missing_caster_removed = missing_caster_map
        .remove_from_map_like_cpp(missing_caster_player_guid, false)
        .unwrap();

    let missing_caster_cleanup = missing_caster_removed.player_viewpoint_cleanup.unwrap();
    assert_eq!(
        missing_caster_cleanup.status,
        PlayerRemoveFromWorldViewpointCleanupStatusLikeCpp::RemovedDynamicObjectViewpoint
    );
    assert_eq!(missing_caster_cleanup.dynamic_object_caster_viewpoint, None);
    let missing_caster_set_viewpoint = missing_caster_cleanup.player_set_viewpoint.unwrap();
    assert_eq!(
        missing_caster_set_viewpoint.status,
        PlayerSetViewpointStatusLikeCpp::Removed
    );
    assert!(missing_caster_set_viewpoint.set_seer_requested);
    assert!(
        missing_caster_map
            .get_typed_dynamic_object(missing_caster_dynamic_object_guid)
            .unwrap()
            .is_caster_viewpoint()
    );

    let mut other_caster_map = test_map();
    let mut removed_player = test_player_for_viewpoint(4930121);
    let removed_player_guid = removed_player.guid();
    let mut other_player = test_player_for_viewpoint(4930122);
    let other_player_guid = other_player.guid();
    let mut dynamic_object = test_dynamic_object_for_viewpoint(4930123);
    let dynamic_object_guid = dynamic_object.world().guid();
    removed_player.set_farsight_object_like_cpp(dynamic_object_guid);
    other_player.set_farsight_object_like_cpp(dynamic_object_guid);
    dynamic_object.set_caster_guid(other_player_guid);
    dynamic_object.bind_to_caster(other_player_guid);
    dynamic_object.set_caster_viewpoint();
    other_caster_map
        .insert_map_object_record(MapObjectRecord::new_player(removed_player).unwrap())
        .unwrap();
    other_caster_map
        .insert_map_object_record(MapObjectRecord::new_player(other_player).unwrap())
        .unwrap();
    other_caster_map
        .insert_map_object_record(MapObjectRecord::new_dynamic_object(dynamic_object).unwrap())
        .unwrap();

    let other_caster_removed = other_caster_map
        .remove_from_map_like_cpp(removed_player_guid, false)
        .unwrap();

    let other_caster_cleanup = other_caster_removed.player_viewpoint_cleanup.unwrap();
    assert_eq!(
        other_caster_cleanup.status,
        PlayerRemoveFromWorldViewpointCleanupStatusLikeCpp::RemovedDynamicObjectViewpoint
    );
    assert_eq!(other_caster_cleanup.dynamic_object_caster_viewpoint, None);
    let other_caster_set_viewpoint = other_caster_cleanup.player_set_viewpoint.unwrap();
    assert_eq!(other_caster_set_viewpoint.player_guid, removed_player_guid);
    assert_eq!(
        other_caster_set_viewpoint.status,
        PlayerSetViewpointStatusLikeCpp::Removed
    );
    assert!(other_caster_set_viewpoint.set_seer_requested);
    assert_eq!(
        other_caster_map
            .get_typed_player(other_player_guid)
            .unwrap()
            .active_data()
            .farsight_object,
        dynamic_object_guid
    );
    assert!(
        other_caster_map
            .get_typed_dynamic_object(dynamic_object_guid)
            .unwrap()
            .is_caster_viewpoint()
    );
}
