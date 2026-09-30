use super::*;

#[test]
fn account_transmog_update_is_not_sent_while_opcode_is_unresolved_like_cpp() {
    let (mut session, _, send_rx) = make_appearance_session();
    session.set_player_guid(Some(ObjectGuid::create_player(1, 1)));
    wow_world::test_fixtures::install_canonical_player_owner_for_test(&mut session, 571, 0);
    appearance_seed_favorite_for_test(&session, 65, FavoriteAppearanceStateLikeCpp::Unchanged);

    session.send_favorite_appearances_like_cpp();

    assert_eq!(
        send_rx.try_recv(),
        Err(flume::TryRecvError::Empty),
        "do not send SMSG_ACCOUNT_TRANSMOG_UPDATE while legacy C++ keeps it at NULL_OPCODE/0xBADD"
    );
}

#[test]
fn item_modified_appearance_helpers_use_cpp_lookup_shapes() {
    let (mut session, _, _) = make_appearance_session();
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
    let (mut session, _, _) = make_appearance_session();
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
    let (mut session, _, _) = make_appearance_session();
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
fn item_spec_class_mask_from_overrides_uses_chr_specialization_like_cpp() {
    let (mut session, _, _) = make_appearance_session();
    session.set_item_spec_override_store(Arc::new(ItemSpecOverrideStore::from_entries([
        ItemSpecOverrideEntry {
            id: 1,
            spec_id: 66,
            item_id: 777,
        },
        ItemSpecOverrideEntry {
            id: 2,
            spec_id: 70,
            item_id: 777,
        },
        ItemSpecOverrideEntry {
            id: 3,
            spec_id: 999,
            item_id: 778,
        },
    ])));
    session.set_chr_specialization_store(Arc::new(ChrSpecializationStore::from_entries([
        ChrSpecializationEntry {
            id: 66,
            class_id: 2,
            order_index: 0,
            role: 0,
        },
        ChrSpecializationEntry {
            id: 70,
            class_id: 3,
            order_index: 0,
            role: 0,
        },
    ])));

    assert_eq!(
        appearance_item_spec_class_mask_for_test(&session, 777),
        Some((1 << 1) | (1 << 2))
    );
    assert_eq!(
        appearance_item_spec_class_mask_for_test(&session, 778),
        Some(0)
    );
    assert_eq!(
        appearance_item_spec_class_mask_for_test(&session, 999),
        None
    );
}
