use super::*;
use wow_entities::Item;

#[test]
fn selected_item_fixture_capture_requires_an_absent_handle() {
    let (mut session, _packet_tx, _send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 5_901);
    let item_guid = ObjectGuid::create_item(1, 9_101);
    session.core.player_guid = Some(player_guid);

    let mut item = Item::new(0);
    item.object_mut().create(item_guid);
    item.object_mut().set_entry(71_009);
    item.set_count(6);
    assert!(
        session
            .inventory
            .insert_inventory_item_object_for_test_like_cpp(item_guid, item)
            .is_none()
    );

    let absent_access = session.core.owned_inventory_access_like_cpp();
    assert!(absent_access.owner_handle_absent_like_cpp());
    let fixture_snapshot = session
        .inventory
        .resolved_inventory_item_object_with_access_like_cpp(&absent_access, item_guid)
        .expect("handle-less fixture should capture its selected item");
    assert_eq!(fixture_snapshot.object().guid(), item_guid);
    assert_eq!(fixture_snapshot.object().entry(), 71_009);
    assert_eq!(fixture_snapshot.data().stack_count, 6);
    assert!(fixture_snapshot.item_data_changes_mask().is_any_set());
    assert!(
        fixture_snapshot
            .object()
            .object_data_changes_mask()
            .is_any_set()
    );

    let manager = shared_canonical_map_manager();
    session.set_canonical_map_manager(Arc::clone(&manager));
    let mut owner = Player::new(None, false);
    owner
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(player_guid);
    let stale_handle = manager
        .lock()
        .unwrap()
        .install_detached_player_like_cpp(Box::new(owner))
        .expect("temporary owner should install");
    session.core.player_handle_like_cpp = Some(stale_handle);
    manager
        .lock()
        .unwrap()
        .retire_player_like_cpp(stale_handle)
        .expect("temporary owner should retire");

    let stale_access = session.core.owned_inventory_access_like_cpp();
    assert!(!stale_access.owner_handle_absent_like_cpp());
    assert!(
        session
            .inventory
            .resolved_inventory_item_object_with_access_like_cpp(&stale_access, item_guid)
            .is_none()
    );
}
