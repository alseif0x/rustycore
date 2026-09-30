use super::*;

#[test]
fn inventory_item_object_uses_template_durability_and_runtime_fields() {
    let (mut session, _, _) = make_session();
    let owner_guid = ObjectGuid::create_player(1, 42);
    let item_guid = ObjectGuid::create_item(1, 900);
    session.lifecycle.total_played_time = 123;
    session.set_item_stats_store(Arc::new(ItemStatsStore::from_sparse_templates([(
        700,
        ItemSparseTemplateEntry {
            max_durability: 55,
            bonding: 0,
            ..inventory_sparse_template_for_test(0)
        },
    )])));

    let mut item = session.make_inventory_item_object(
        item_guid,
        700,
        owner_guid,
        3,
        44,
        ItemContext::Vendor,
        35,
    );
    item.set_state(ItemUpdateState::Unchanged);
    session.insert_inventory_item_object(item);

    let stored = session.inventory_item_objects.get(&item_guid).unwrap();
    assert_eq!(stored.object().entry(), 700);
    assert_eq!(stored.data().owner, owner_guid);
    assert_eq!(stored.data().contained_in, owner_guid);
    assert_eq!(stored.data().stack_count, 3);
    assert_eq!(stored.data().max_durability, 55);
    assert_eq!(stored.data().durability, 44);
    assert_eq!(stored.data().context, ItemContext::Vendor as i32);
    assert_eq!(stored.slot(), 35);
    assert_eq!(stored.update_state(), ItemUpdateState::Unchanged);

    session.set_inventory_item_object_slot(item_guid, 36);
    assert_eq!(
        session
            .inventory_item_objects
            .get(&item_guid)
            .unwrap()
            .slot(),
        36
    );
    assert!(session.remove_inventory_item_object(item_guid).is_some());
    assert!(!session.inventory_item_objects.contains_key(&item_guid));
}
#[test]
fn canonical_player_logout_cleanup_removes_player_before_session_inventory_like_cpp() {
    run_canonical_player_owner_test(|| {
        let (mut session, _, _) = make_session();
        let canonical = shared_canonical_map_manager();
        let player_guid = ObjectGuid::create_player(1, 46);
        let item_guid = ObjectGuid::create_item(1, 901);

        session.set_canonical_map_manager(Arc::clone(&canonical));
        session.set_player_guid(Some(player_guid));
        session.player_name = Some("LogoutMap".into());
        session.player_position = Some(Position::new(1.0, 2.0, 3.0, 0.0));
        session.current_map_id = 571;
        session
            .player_item_test_fixture_like_cpp
            .inventory_items
            .insert(
                23,
                InventoryItem {
                    guid: item_guid,
                    entry_id: 700,
                    db_guid: 901,
                    inventory_type: None,
                },
            );
        let item = session.make_inventory_item_object(
            item_guid,
            700,
            player_guid,
            1,
            0,
            ItemContext::None,
            23,
        );
        session.insert_inventory_item_object(item);

        insert_session_player_into_canonical_map_like_cpp(&session, &canonical, 571, 0);
        assert!(session.adopt_registered_canonical_player_fixture_like_cpp());

        assert!(
            canonical
                .lock()
                .unwrap()
                .find_map(571, 0)
                .unwrap()
                .map()
                .get_typed_player(player_guid)
                .is_some()
        );

        session.cleanup_shared_runtime_state();

        assert!(
            canonical
                .lock()
                .unwrap()
                .find_map(571, 0)
                .unwrap()
                .map()
                .get_typed_player(player_guid)
                .is_none()
        );
        assert!(
            session
                .player_item_test_fixture_like_cpp
                .inventory_items
                .is_empty()
        );
        assert!(session.inventory_item_objects.is_empty());
    });
}
