use super::*;

#[test]
fn on_item_added_adds_heirloom_and_permanent_appearance_like_cpp() {
    let (mut session, _, _) = make_appearance_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 86);
    let player_position = Position::new(10.0, 0.0, 0.0, 0.0);
    let item_guid = ObjectGuid::create_item(1, 86_777);
    session.set_player_guid(Some(player_guid));
    set_loaded_player_identity_like_cpp(&mut session, 571, 1, 1, 80, 0);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    add_canonical_test_player_on_map(&canonical, player_guid, player_position, 571, 0);
    wow_world::test_fixtures::appearance_adopt_registered_player_for_test(&mut session);
    grant_learned_weapon_proficiency_like_cpp(
        &mut session,
        1 << (ItemSubClassWeapon::Sword as u32),
    );
    session.set_heirloom_store(Arc::new(HeirloomStore::from_entries([HeirloomEntry {
        id: 1,
        source_text: "collection item".to_string(),
        item_id: 44_000,
        legacy_upgraded_item_id: 0,
        static_upgraded_item_id: 0,
        source_type_enum: 0,
        flags: 0,
        legacy_item_id: 0,
        upgrade_item_id: [0; 6],
        upgrade_item_bonus_list_id: [0; 6],
    }])));
    session.set_item_modified_appearance_store(Arc::new(
        ItemModifiedAppearanceStore::from_entries([ItemModifiedAppearanceEntry {
            id: 65,
            item_id: 44_000,
            item_appearance_modifier_id: 0,
            item_appearance_id: 9_000,
            order_index: 0,
            transmog_source_type_enum: 0,
        }]),
    ));
    install_transmog_can_add_test_item(
        &mut session,
        44_000,
        ItemClass::Weapon,
        ItemSubClassWeapon::Sword as u8,
        InventoryType::Weapon,
        ItemQuality::Uncommon,
        [0, 0, 0, 0],
        0,
    );
    mutate_canonical_player_for_test(&session, |player| player.clear_data_changes());
    let mut item = wow_world::test_fixtures::make_inventory_item_object_for_test(&session, 
        item_guid,
        44_000,
        player_guid,
        1,
        0,
        ItemContext::None,
        23,
    );
    item.set_item_flag(ItemFieldFlags::SOULBOUND);

    let updates = appearance_on_item_added_for_test(&mut session, &item);

    assert_eq!(updates.len(), 2);
    assert_eq!(appearance_heirloom_rows_for_test(&session), vec![(44_000, 0)]);
    assert!(appearance_has_permanent_for_test(&session, 65));
    let canonical_fields = mutate_canonical_player_for_test(&session, |player| {
            (
                player.heirlooms_like_cpp().to_vec(),
                player.heirloom_flags_like_cpp().to_vec(),
                player.transmog_blocks_like_cpp().to_vec(),
            )
        })
        .unwrap();
    assert_eq!(
        canonical_fields,
        (vec![44_000], vec![0], vec![0, 0, 1 << 1])
    );
}

#[test]
fn on_item_added_records_refundable_appearance_as_temporary_like_cpp() {
    let (mut session, _, _) = make_appearance_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 87);
    let player_position = Position::new(10.0, 0.0, 0.0, 0.0);
    let item_guid = ObjectGuid::create_item(1, 87_777);
    session.set_player_guid(Some(player_guid));
    set_loaded_player_identity_like_cpp(&mut session, 571, 1, 1, 80, 0);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    add_canonical_test_player_on_map(&canonical, player_guid, player_position, 571, 0);
    wow_world::test_fixtures::appearance_adopt_registered_player_for_test(&mut session);
    grant_learned_weapon_proficiency_like_cpp(
        &mut session,
        1 << (ItemSubClassWeapon::Sword as u32),
    );
    session.set_item_modified_appearance_store(Arc::new(
        ItemModifiedAppearanceStore::from_entries([ItemModifiedAppearanceEntry {
            id: 96,
            item_id: 44_100,
            item_appearance_modifier_id: 0,
            item_appearance_id: 9_100,
            order_index: 0,
            transmog_source_type_enum: 0,
        }]),
    ));
    install_transmog_can_add_test_item(
        &mut session,
        44_100,
        ItemClass::Weapon,
        ItemSubClassWeapon::Sword as u8,
        InventoryType::Weapon,
        ItemQuality::Uncommon,
        [0, 0, 0, 0],
        0,
    );
    mutate_canonical_player_for_test(&session, |player| player.clear_data_changes());
    let mut item = wow_world::test_fixtures::make_inventory_item_object_for_test(&session, 
        item_guid,
        44_100,
        player_guid,
        1,
        0,
        ItemContext::Vendor,
        23,
    );
    item.set_item_flag(ItemFieldFlags::SOULBOUND | ItemFieldFlags::REFUNDABLE);

    let updates = appearance_on_item_added_for_test(&mut session, &item);

    assert_eq!(updates.len(), 1);
    assert_eq!(session.has_item_appearance_like_cpp(96), (true, true));
    assert!(!appearance_has_permanent_for_test(&session, 96));
    assert_eq!(
        session.items_providing_temporary_appearance_like_cpp(96),
        HashSet::from([item_guid])
    );
}
