use super::*;

#[test]
fn temporary_item_appearance_tracks_conditional_transmog_like_cpp() {
    let (mut session, _, _) = make_appearance_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 76);
    let player_position = Position::new(10.0, 0.0, 0.0, 0.0);
    let item_guid_1 = ObjectGuid::create_item(1, 901);
    let item_guid_2 = ObjectGuid::create_item(1, 902);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    attach_appearance_player_for_test(
        &mut session,
        player_guid,
        "ConditionalTransmogTester".to_string(),
        player_position,
        571,
        1,
        1,
        80,
        0,
    );
    add_canonical_test_player_on_map(&canonical, player_guid, player_position, 571, 0);
    wow_world::test_fixtures::appearance_adopt_registered_player_for_test(&mut session);
    mutate_canonical_player_for_test(&session, |player| player.clear_data_changes());

    let update = session
        .add_temporary_item_appearance_like_cpp(65, item_guid_1)
        .expect("first temporary provider should add conditional transmog");
    let active = update
        .active_player_data
        .expect("conditional transmog should mark active player data");
    assert_eq!(active.values.conditional_transmog, vec![65]);
    assert_eq!(
        active.values.conditional_transmog_update_mask,
        Some(vec![1])
    );
    assert_eq!(session.has_item_appearance_like_cpp(65), (true, true));

    assert!(
        session
            .add_temporary_item_appearance_like_cpp(65, item_guid_2)
            .is_none()
    );
    assert_eq!(
        session.items_providing_temporary_appearance_like_cpp(65),
        HashSet::from([item_guid_1, item_guid_2])
    );

    assert!(
        session
            .remove_temporary_item_appearance_like_cpp(65, item_guid_1)
            .is_none()
    );
    assert_eq!(session.has_item_appearance_like_cpp(65), (true, true));

    let update = session
        .remove_temporary_item_appearance_like_cpp(65, item_guid_2)
        .expect("last temporary provider should remove conditional transmog");
    let active = update
        .active_player_data
        .expect("conditional transmog removal should mark active player data");
    assert_eq!(active.values.conditional_transmog, Vec::<i32>::new());
    assert_eq!(
        active.values.conditional_transmog_update_mask,
        Some(vec![1])
    );
    assert_eq!(session.has_item_appearance_like_cpp(65), (false, false));

    session
        .add_temporary_item_appearance_like_cpp(65, item_guid_1)
        .expect("temporary appearance can be re-added");
    let update = session
        .add_item_appearance_like_cpp(65)
        .expect("permanent appearance should remove matching temporary appearance");
    let active = update
        .active_player_data
        .expect("permanent appearance should mark active player data");
    assert_eq!(active.values.conditional_transmog, Vec::<i32>::new());
    assert_eq!(session.has_item_appearance_like_cpp(65), (true, false));
    assert!(
        session
            .items_providing_temporary_appearance_like_cpp(65)
            .is_empty()
    );
}
