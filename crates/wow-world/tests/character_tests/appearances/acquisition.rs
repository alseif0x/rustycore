use super::*;

#[test]
fn add_item_appearance_resizes_blocks_and_marks_flag_like_cpp() {
    let (mut session, _, _) = make_appearance_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 71);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    let player_position = Position::new(10.0, 0.0, 0.0, 0.0);
    attach_appearance_player_for_test(
        &mut session,
        player_guid,
        "TransmogTester".to_string(),
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
        .add_item_appearance_like_cpp(65)
        .expect("represented current player should receive transmog appearance");
    let active = update
        .active_player_data
        .expect("transmog appearance should mark active player data");

    assert!(appearance_has_permanent_for_test(&session, 65));
    assert_eq!(active.values.transmog, vec![0, 0, 1 << 1]);
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
    assert!(session.add_item_appearance_like_cpp(65).is_none());
}

#[test]
fn add_item_appearance_for_item_resolves_modified_appearance_like_cpp() {
    let (mut session, _, _) = make_appearance_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 77);
    let player_position = Position::new(10.0, 0.0, 0.0, 0.0);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    attach_appearance_player_for_test(
        &mut session,
        player_guid,
        "TransmogItemResolverTester".to_string(),
        player_position,
        571,
        1,
        1,
        80,
        0,
    );
    add_canonical_test_player_on_map(&canonical, player_guid, player_position, 571, 0);
    wow_world::test_fixtures::appearance_adopt_registered_player_for_test(&mut session);
    grant_learned_weapon_proficiency_like_cpp(
        &mut session,
        1 << (ItemSubClassWeapon::Sword as u32),
    );
    session.set_item_modified_appearance_store(Arc::new(
        ItemModifiedAppearanceStore::from_entries([ItemModifiedAppearanceEntry {
            id: 65,
            item_id: 777,
            item_appearance_modifier_id: 2,
            item_appearance_id: 9_000,
            order_index: 0,
            transmog_source_type_enum: 0,
        }]),
    ));
    install_transmog_can_add_test_item(
        &mut session,
        777,
        ItemClass::Weapon,
        ItemSubClassWeapon::Sword as u8,
        InventoryType::Weapon,
        ItemQuality::Uncommon,
        [0, 0, 0, 0],
        0,
    );
    mutate_canonical_player_for_test(&session, |player| player.clear_data_changes());

    let update = session
        .add_item_appearance_for_item_like_cpp(777, 2)
        .expect("represented item appearance should resolve by item/modifier");
    let active = update
        .active_player_data
        .expect("resolved item appearance should mark active player data");

    assert!(appearance_has_permanent_for_test(&session, 65));
    assert_eq!(active.values.transmog, vec![0, 0, 1 << 1]);
    assert!(
        session
            .add_item_appearance_for_item_like_cpp(777, 2)
            .is_none()
    );
    assert!(
        session
            .add_item_appearance_for_item_like_cpp(778, 2)
            .is_none()
    );
}
