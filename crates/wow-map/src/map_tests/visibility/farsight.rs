use super::*;
use wow_entities::DynamicObjectType;

#[test]
fn farsight_dynamic_object_create_inserts_focus_and_sets_viewpoint_like_cpp() {
    let mut map = test_map();
    let player = test_player_for_viewpoint(4280101);
    let player_guid = player.guid();
    map.insert_map_object_record(MapObjectRecord::new_player(player).unwrap())
        .unwrap();

    let outcome = create_farsight_focus_for_tests(&mut map, player_guid);

    assert_eq!(
        outcome.status,
        FarsightDynamicObjectCreateStatusLikeCpp::Created
    );
    assert_eq!(outcome.caster_player_guid, player_guid);
    assert_eq!(outcome.low_guid, Some(1));
    let dynamic_guid = outcome.dynamic_object_guid.unwrap();
    assert_eq!(dynamic_guid.high_type(), HighGuid::DynamicObject);
    assert_ne!(dynamic_guid.counter(), 12_345);
    assert_eq!(
        map.get_max_low_guid_like_cpp(HighGuid::DynamicObject)
            .unwrap(),
        2
    );
    let add_to_map = outcome.add_to_map.unwrap();
    assert!(add_to_map.inserted);
    assert!(add_to_map.inserted_into_cell);
    assert!(!add_to_map.already_in_world);

    let dynamic_object = map.get_typed_dynamic_object(dynamic_guid).unwrap();
    assert_eq!(dynamic_object.world().guid(), dynamic_guid);
    assert_eq!(dynamic_object.world().map_id(), 571);
    assert_eq!(dynamic_object.world().instance_id(), 7);
    assert_eq!(
        dynamic_object.world().position(),
        Position::new(100.0, 200.0, 30.0, 1.5)
    );
    assert!(dynamic_object.world().object().is_in_world());
    assert!(dynamic_object.world().is_active());
    assert_eq!(dynamic_object.world().object().entry(), 12_345);
    assert_eq!(dynamic_object.world().object().scale(), 1.0);
    assert_eq!(dynamic_object.caster_guid(), player_guid);
    assert_eq!(dynamic_object.bound_caster(), Some(player_guid));
    assert_eq!(
        dynamic_object.data().dynamic_object_type,
        DynamicObjectType::FarsightFocus as u8
    );
    assert_eq!(dynamic_object.data().spell_visual_id, 678);
    assert_eq!(dynamic_object.spell_id(), 12_345);
    assert_eq!(dynamic_object.radius(), 42.5);
    assert_eq!(dynamic_object.data().cast_time_ms, 987_654);
    assert_eq!(dynamic_object.duration_ms(), 30_000);
    assert!(dynamic_object.is_caster_viewpoint());
    assert_eq!(
        map.get_typed_player(player_guid)
            .unwrap()
            .active_data()
            .farsight_object,
        dynamic_guid
    );
    let viewpoint = outcome.caster_viewpoint.unwrap();
    assert_eq!(viewpoint.dynamic_object_guid, dynamic_guid);
    assert_eq!(
        viewpoint.status,
        DynamicObjectCasterViewpointStatusLikeCpp::CasterPlayerResolved
    );
    assert_eq!(
        viewpoint.player_set_viewpoint.status,
        PlayerSetViewpointStatusLikeCpp::Applied
    );
    assert!(viewpoint.player_set_viewpoint.update_visibility_requested);
    assert!(viewpoint.player_set_viewpoint.set_seer_requested);
}
#[test]
fn farsight_dynamic_object_create_invalid_destination_preserves_no_mutation_like_cpp() {
    let invalid_destinations = [
        Position::new(f32::NAN, 200.0, 30.0, 1.5),
        Position::new(100.0, 200.0, f32::NAN, 1.5),
        Position::new(100.0, 200.0, Position::MAP_HALFSIZE_LIKE_CPP, 1.5),
        Position::new(100.0, 200.0, 30.0, f32::NAN),
        Position::new(100.0, 200.0, 30.0, f32::INFINITY),
    ];

    for (index, dest) in invalid_destinations.into_iter().enumerate() {
        let mut map = test_map();
        let player = test_player_for_viewpoint(4280501 + index as i64);
        let player_guid = player.guid();
        map.insert_map_object_record(MapObjectRecord::new_player(player).unwrap())
            .unwrap();

        let outcome = map.create_farsight_dynamic_object_like_cpp(
            player_guid,
            12_345,
            678,
            dest,
            42.5,
            30_000,
            987_654,
            1,
            7,
        );

        assert_eq!(
            outcome.status,
            FarsightDynamicObjectCreateStatusLikeCpp::InvalidDestination
        );
        assert_eq!(map.entity_world.len(), 1);
        assert_eq!(
            map.get_max_low_guid_like_cpp(HighGuid::DynamicObject)
                .unwrap(),
            1
        );
        assert_eq!(
            map.get_typed_player(player_guid)
                .unwrap()
                .active_data()
                .farsight_object,
            ObjectGuid::EMPTY
        );
    }
}
#[test]
fn farsight_dynamic_object_create_reports_viewpoint_no_mutation_without_panicking_like_cpp() {
    let mut map = test_map();
    let mut player = test_player_for_viewpoint(4280601);
    let player_guid = player.guid();
    let existing_guid = guid(HighGuid::Creature, 4280609);
    player.set_farsight_object_like_cpp(existing_guid);
    map.insert_map_object_record(MapObjectRecord::new_player(player).unwrap())
        .unwrap();

    let outcome = create_farsight_focus_for_tests(&mut map, player_guid);

    assert_eq!(
        outcome.status,
        FarsightDynamicObjectCreateStatusLikeCpp::Created
    );
    let dynamic_guid = outcome.dynamic_object_guid.unwrap();
    let viewpoint = outcome.caster_viewpoint.unwrap();
    assert_eq!(
        viewpoint.player_set_viewpoint.status,
        PlayerSetViewpointStatusLikeCpp::AlreadyHasViewpoint
    );
    assert!(!viewpoint.player_set_viewpoint.update_visibility_requested);
    assert!(!viewpoint.player_set_viewpoint.set_seer_requested);
    assert!(viewpoint.dynamic_object_viewpoint_toggled);
    assert_eq!(
        map.get_typed_player(player_guid)
            .unwrap()
            .active_data()
            .farsight_object,
        existing_guid
    );
    assert!(
        map.get_typed_dynamic_object(dynamic_guid)
            .unwrap()
            .is_caster_viewpoint()
    );
}
