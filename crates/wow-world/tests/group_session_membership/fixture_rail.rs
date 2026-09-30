//! Feature-enabled integration checks for the ordinary and opted-in fixture rails.

use super::*;

#[test]
fn ordinary_group_feature_calls_do_not_enable_character_fixture_or_install_owner() {
    let (_pkt_tx, pkt_rx) = flume::bounded(100);
    let (send_tx, _send_rx) = flume::unbounded();
    let mut session = WorldSession::new(
        1,
        "TestAccount".into(),
        0,
        2,
        9,
        54261,
        vec![0u8; 40],
        "esES".into(),
        pkt_rx,
        send_tx,
    );
    assert!(!session.character_lifecycle_fixture_is_enabled_for_test());
    assert_eq!(session.player_guid(), None);
    assert!(!session.group_reset_update_sequence_for_test());
    assert_eq!(session.group_next_update_sequence_for_test(0), Some(0));
    assert!(!session.group_apply_leader_flag_for_test());
    session.character_attach_player_controller_for_test(
        ObjectGuid::create_player(1, 42),
        "Tester".into(),
        Position::ZERO,
        571,
        1,
        1,
        80,
        0,
    );
    assert_eq!(session.player_guid(), None);
    assert_eq!(session.character_player_handle_for_test(), None);
    assert!(!session.character_lifecycle_fixture_is_enabled_for_test());
}

#[test]
fn opted_in_group_fixture_without_guid_does_not_install_a_canonical_owner() {
    let (mut session, _, _) = make_session();
    assert!(session.character_lifecycle_fixture_is_enabled_for_test());
    assert_eq!(session.player_guid(), None);
    assert!(!session.group_reset_update_sequence_for_test());
    assert_eq!(session.group_next_update_sequence_for_test(0), Some(0));
    assert!(!session.group_apply_leader_flag_for_test());
    assert!(
        session
            .character_ensure_canonical_world_map_for_current_player_for_test()
            .is_none()
    );
    assert_eq!(session.character_player_handle_for_test(), None);
}

#[test]
fn stale_group_fixture_forwards_leave_the_replacement_owner_unchanged() {
    let (mut session, _, _) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 5_572);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_map_store(canonical_player_transfer_test_map_store_like_cpp());
    session.character_attach_player_controller_for_test(
        player_guid,
        "GroupOwner".to_string(),
        Position::new(3700.0, 1500.0, 120.0, 0.0),
        571,
        1,
        1,
        80,
        0,
    );
    session
        .character_ensure_canonical_world_map_for_current_player_for_test()
        .expect("initial world map");
    let old_handle = session
        .character_player_handle_for_test()
        .expect("canonical handle");
    assert!(session.character_remove_current_player_from_canonical_current_map_for_test());

    let mut replacement = Box::new(Player::new(Some(2), false));
    replacement
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(player_guid);
    replacement.set_player_flag(PLAYER_FLAGS_GROUP_LEADER_LIKE_CPP);
    let replacement_handle = canonical
        .lock()
        .unwrap()
        .install_detached_player_like_cpp(replacement)
        .expect("replacement owner");

    assert_eq!(session.character_player_handle_for_test(), Some(old_handle));
    assert!(!session.group_reset_update_sequence_for_test());
    assert_eq!(session.group_next_update_sequence_for_test(0), None);
    assert!(!session.group_apply_leader_flag_for_test());
    assert_eq!(
        canonical
            .lock()
            .unwrap()
            .with_player_like_cpp(replacement_handle, |player| {
                player.has_player_flag(PLAYER_FLAGS_GROUP_LEADER_LIKE_CPP)
            }),
        Some(true)
    );
}
