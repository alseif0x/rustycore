use super::*;

#[test]
fn item_modified_appearance_helpers_use_cpp_lookup_shapes() {
    let (mut session, _, _) = make_session();
    session.set_item_appearance_store(Arc::new(ItemAppearanceStore::from_entries([
        ItemAppearanceEntry {
            id: 1000,
            display_type: 0,
            item_display_info_id: 555,
            default_icon_file_data_id: 0,
            ui_order: 0,
        },
        ItemAppearanceEntry {
            id: 1001,
            display_type: 0,
            item_display_info_id: 777,
            default_icon_file_data_id: 0,
            ui_order: 0,
        },
    ])));
    session.set_item_modified_appearance_store(Arc::new(
        ItemModifiedAppearanceStore::from_entries([
            ItemModifiedAppearanceEntry {
                id: 10,
                item_id: 100,
                item_appearance_modifier_id: 0,
                item_appearance_id: 1000,
                order_index: 0,
                transmog_source_type_enum: 0,
            },
            ItemModifiedAppearanceEntry {
                id: 11,
                item_id: 100,
                item_appearance_modifier_id: 2,
                item_appearance_id: 1001,
                order_index: 0,
                transmog_source_type_enum: 0,
            },
        ]),
    ));

    assert_eq!(session.item_modified_appearance_ref(11), Some((100, 2)));
    assert_eq!(session.item_modified_appearance_for_item(100, 2), Some(11));
    assert_eq!(session.item_modified_appearance_for_item(100, 9), Some(10));
    assert_eq!(session.item_modified_appearance_for_item(101, 0), None);
    assert_eq!(session.item_display_id(100, 2), Some(777));
    assert_eq!(session.item_display_id(100, 9), Some(555));
    assert_eq!(session.item_display_id(101, 0), None);
}
#[test]
fn transmog_set_items_helper_uses_cpp_lookup_shape() {
    let (mut session, _, _) = make_session();
    session.set_transmog_set_item_store(Arc::new(TransmogSetItemStore::from_entries([
        TransmogSetItemEntry {
            id: 1,
            transmog_set_id: 70,
            item_modified_appearance_id: 1000,
            flags: 0,
        },
        TransmogSetItemEntry {
            id: 2,
            transmog_set_id: 71,
            item_modified_appearance_id: 2000,
            flags: 0,
        },
        TransmogSetItemEntry {
            id: 3,
            transmog_set_id: 70,
            item_modified_appearance_id: 1001,
            flags: 0,
        },
    ])));

    let items = session
        .transmog_set_items_like_cpp(70)
        .expect("transmog set should have indexed items");
    assert_eq!(
        items
            .iter()
            .map(|item| item.item_modified_appearance_id)
            .collect::<Vec<_>>(),
        vec![1000, 1001]
    );
    assert!(session.transmog_set_items_like_cpp(99).is_none());
}
#[test]
fn transmog_set_item_modified_appearances_skip_missing_like_cpp() {
    let (mut session, _, _) = make_session();
    session.set_transmog_set_item_store(Arc::new(TransmogSetItemStore::from_entries([
        TransmogSetItemEntry {
            id: 1,
            transmog_set_id: 70,
            item_modified_appearance_id: 1000,
            flags: 0,
        },
        TransmogSetItemEntry {
            id: 2,
            transmog_set_id: 70,
            item_modified_appearance_id: 1001,
            flags: 0,
        },
    ])));
    session.set_item_modified_appearance_store(Arc::new(
        ItemModifiedAppearanceStore::from_entries([ItemModifiedAppearanceEntry {
            id: 1001,
            item_id: 777,
            item_appearance_modifier_id: 0,
            item_appearance_id: 9000,
            order_index: 0,
            transmog_source_type_enum: 0,
        }]),
    ));

    let appearances = session.transmog_set_item_modified_appearances_like_cpp(70);
    assert_eq!(appearances.len(), 1);
    assert_eq!(appearances[0].id, 1001);
    assert_eq!(appearances[0].item_id, 777);
    assert!(
        session
            .transmog_set_item_modified_appearances_like_cpp(99)
            .is_empty()
    );
}
#[test]
fn add_item_appearance_resizes_blocks_and_marks_flag_like_cpp() {
    let (mut session, _, _) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 71);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    let player_position = Position::new(10.0, 0.0, 0.0, 0.0);
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "TransmogTester".to_string(),
        player_position,
        571,
        1,
        1,
        80,
        0,
    ));
    add_canonical_test_player_on_map(&canonical, player_guid, player_position, 571, 0);
    session.mutate_canonical_player_like_cpp(|player| player.clear_data_changes());

    let update = session
        .add_item_appearance_like_cpp(65)
        .expect("represented current player should receive transmog appearance");
    let active = update
        .active_player_data
        .expect("transmog appearance should mark active player data");

    assert!(session.represented_has_item_appearance_like_cpp(65));
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
    let (mut session, _, _) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 77);
    let player_position = Position::new(10.0, 0.0, 0.0, 0.0);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "TransmogItemResolverTester".to_string(),
        player_position,
        571,
        1,
        1,
        80,
        0,
    ));
    add_canonical_test_player_on_map(&canonical, player_guid, player_position, 571, 0);
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
    session.mutate_canonical_player_like_cpp(|player| player.clear_data_changes());

    let update = session
        .add_item_appearance_for_item_like_cpp(777, 2)
        .expect("represented item appearance should resolve by item/modifier");
    let active = update
        .active_player_data
        .expect("resolved item appearance should mark active player data");

    assert!(session.represented_has_item_appearance_like_cpp(65));
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
#[test]
fn on_item_added_adds_heirloom_and_permanent_appearance_like_cpp() {
    let (mut session, _, _) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 86);
    let player_position = Position::new(10.0, 0.0, 0.0, 0.0);
    let item_guid = ObjectGuid::create_item(1, 86_777);
    session.set_player_guid(Some(player_guid));
    session.set_loaded_player_identity_like_cpp(571, 1, 1, 80, 0);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    add_canonical_test_player_on_map(&canonical, player_guid, player_position, 571, 0);
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
    session.mutate_canonical_player_like_cpp(|player| player.clear_data_changes());
    let mut item = session.make_inventory_item_object(
        item_guid,
        44_000,
        player_guid,
        1,
        0,
        ItemContext::None,
        23,
    );
    item.set_item_flag(ItemFieldFlags::SOULBOUND);

    let updates = session.on_item_added_to_collection_like_cpp(&item);

    assert_eq!(updates.len(), 2);
    assert_eq!(session.account_heirloom_rows_like_cpp(), vec![(44_000, 0)]);
    assert!(session.represented_has_item_appearance_like_cpp(65));
    let canonical_fields = session
        .mutate_canonical_player_like_cpp(|player| {
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
    let (mut session, _, _) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 87);
    let player_position = Position::new(10.0, 0.0, 0.0, 0.0);
    let item_guid = ObjectGuid::create_item(1, 87_777);
    session.set_player_guid(Some(player_guid));
    session.set_loaded_player_identity_like_cpp(571, 1, 1, 80, 0);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    add_canonical_test_player_on_map(&canonical, player_guid, player_position, 571, 0);
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
    session.mutate_canonical_player_like_cpp(|player| player.clear_data_changes());
    let mut item = session.make_inventory_item_object(
        item_guid,
        44_100,
        player_guid,
        1,
        0,
        ItemContext::Vendor,
        23,
    );
    item.set_item_flag(ItemFieldFlags::SOULBOUND | ItemFieldFlags::REFUNDABLE);

    let updates = session.on_item_added_to_collection_like_cpp(&item);

    assert_eq!(updates.len(), 1);
    assert_eq!(session.has_item_appearance_like_cpp(96), (true, true));
    assert!(!session.represented_has_item_appearance_like_cpp(96));
    assert_eq!(
        session.items_providing_temporary_appearance_like_cpp(96),
        HashSet::from([item_guid])
    );
}
