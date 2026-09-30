use super::*;

#[test]
fn appearance_feature_without_owner_does_not_enable_collection_fallback() {
    let (mut session, _, _) = make_appearance_session();
    assert_eq!(session.player_guid(), None);
    assert_eq!(session.has_item_appearance_like_cpp(65), (false, false));
    assert!(wow_world::test_fixtures::appearance_save_plan_for_test(&mut session).is_none());
    session.set_player_guid(Some(ObjectGuid::create_player(1, 91)));
    wow_world::test_fixtures::enable_ownerless_inventory_snapshots_for_test(&mut session);
    assert_eq!(session.has_item_appearance_like_cpp(65), (false, false));
    assert!(!session.set_appearance_is_favorite_like_cpp(65, true));
    assert!(session.add_temporary_item_appearance_like_cpp(
        65, ObjectGuid::create_item(1, 900),
    ).is_none());
    assert!(session.items_providing_temporary_appearance_like_cpp(65).is_empty());
}

#[test]
fn appearance_feature_stale_some_handle_never_uses_a_fallback() {
    let (mut session, _, _) = make_appearance_session();
    let guid = ObjectGuid::create_player(1, 92);
    session.set_player_guid(Some(guid));
    wow_world::test_fixtures::install_canonical_player_owner_for_test(&mut session, 571, 0);
    session.add_item_appearance_like_cpp(65).unwrap();
    let handle = wow_world::test_fixtures::appearance_owner_handle_for_test(&session).unwrap();
    let canonical = wow_world::test_fixtures::canonical_map_manager_for_test(&session).unwrap();
    assert!(canonical.lock().unwrap().retire_player_like_cpp(handle).is_some());
    assert!(wow_world::test_fixtures::appearance_owner_handle_for_test(&session).is_some());
    wow_world::test_fixtures::enable_ownerless_inventory_snapshots_for_test(&mut session);
    assert_eq!(session.has_item_appearance_like_cpp(65), (false, false));
    assert!(wow_world::test_fixtures::appearance_save_plan_for_test(&mut session).is_none());
    assert!(!session.set_appearance_is_favorite_like_cpp(65, true));
}

#[test]
fn appearance_feature_reads_canonical_owner_and_settles_favorites_before_save() {
    let (mut session, _, send_rx) = make_appearance_session();
    session.set_player_guid(Some(ObjectGuid::create_player(1, 93)));
    wow_world::test_fixtures::install_canonical_player_owner_for_test(&mut session, 571, 0);
    wow_world::test_fixtures::enable_ownerless_inventory_snapshots_for_test(&mut session);
    session.add_item_appearance_like_cpp(65).unwrap();
    assert!(session.set_appearance_is_favorite_like_cpp(65, true));
    assert_eq!(session.has_item_appearance_like_cpp(65), (true, false));
    let plan = wow_world::test_fixtures::appearance_save_plan_for_test(&mut session).unwrap();
    assert_eq!(plan.appearance_blocks, vec![(2, 2)]);
    assert_eq!(plan.favorite_inserts, vec![65]);
    let next = wow_world::test_fixtures::appearance_save_plan_for_test(&mut session).unwrap();
    assert!(next.favorite_inserts.is_empty());
    assert_eq!(send_rx.try_recv(), Err(flume::TryRecvError::Empty));
}
