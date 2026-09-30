use super::*;

#[test]
fn add_transmog_set_skips_missing_and_marks_appearances_like_cpp() {
    let (mut session, _, _) = make_appearance_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 72);
    let player_position = Position::new(10.0, 0.0, 0.0, 0.0);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    attach_appearance_player_for_test(
        &mut session,
        player_guid,
        "TransmogSetTester".to_string(),
        player_position,
        571,
        1,
        1,
        80,
        0,
    );
    add_canonical_test_player_on_map(&canonical, player_guid, player_position, 571, 0);
    wow_world::test_fixtures::appearance_adopt_registered_player_for_test(&mut session);
    session.set_transmog_set_item_store(Arc::new(TransmogSetItemStore::from_entries([
        TransmogSetItemEntry {
            id: 1,
            transmog_set_id: 70,
            item_modified_appearance_id: 0,
            flags: 0,
        },
        TransmogSetItemEntry {
            id: 2,
            transmog_set_id: 70,
            item_modified_appearance_id: 999,
            flags: 0,
        },
        TransmogSetItemEntry {
            id: 3,
            transmog_set_id: 70,
            item_modified_appearance_id: 65,
            flags: 0,
        },
    ])));
    session.set_item_modified_appearance_store(Arc::new(
        ItemModifiedAppearanceStore::from_entries([
            ItemModifiedAppearanceEntry {
                id: 0,
                item_id: 777,
                item_appearance_modifier_id: 0,
                item_appearance_id: 9000,
                order_index: 0,
                transmog_source_type_enum: 0,
            },
            ItemModifiedAppearanceEntry {
                id: 65,
                item_id: 778,
                item_appearance_modifier_id: 0,
                item_appearance_id: 9001,
                order_index: 0,
                transmog_source_type_enum: 0,
            },
        ]),
    ));
    mutate_canonical_player_for_test(&session, |player| player.clear_data_changes());

    let update = session
        .add_transmog_set_like_cpp(70)
        .expect("represented current player should receive transmog set update");
    let active = update
        .active_player_data
        .expect("transmog set should mark active player data");

    assert!(appearance_has_permanent_for_test(&session, 0));
    assert!(appearance_has_permanent_for_test(&session, 65));
    assert!(!appearance_has_permanent_for_test(&session, 999));
    assert_eq!(active.values.transmog, vec![1, 0, 1 << 1]);
    assert_eq!(active.values.transmog_update_mask, Some(vec![0b111]));
    assert!(
        active
            .mask
            .is_set(wow_entities::ACTIVE_PLAYER_DATA_PARENT_BIT)
    );
    assert!(
        active
            .mask
            .is_set(wow_entities::ACTIVE_PLAYER_DATA_TRANSMOG_BIT)
    );
    assert!(session.add_transmog_set_like_cpp(99).is_none());
}
