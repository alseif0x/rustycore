//! Original Group Session application cases.

use super::*;

#[test]
fn represented_group_leader_flag_is_removed_for_non_leader_like_cpp() {
    let (mut session, _, player_guid) = session_with_canonical_player_for_away_like_cpp();
    mutate_canonical_player_for_test(&session, |player| {
        player.set_player_flag(PLAYER_FLAGS_GROUP_LEADER_LIKE_CPP);
    })
    .unwrap();
    let leader_guid = ObjectGuid::create_player(1, 99);
    let group_registry = Arc::new(GroupRegistry::default());
    let mut group = GroupInfo::new(leader_guid);
    group.add_member(player_guid);
    let group_guid = group.group_guid;
    group_registry.register_group_like_cpp(group_guid, group);
    set_group_guid_for_test_like_cpp(&mut session, Some(group_guid));
    session.set_group_registry(group_registry, Arc::new(PendingInvites::default()));

    assert!(session.group_apply_leader_flag_for_test());

    assert_eq!(
        session.character_canonical_player_has_player_flag_for_test(
            player_guid,
            PLAYER_FLAGS_GROUP_LEADER_LIKE_CPP
        ),
        Some(false)
    );
}

#[test]
fn canonical_player_group_reference_follows_detached_and_stale_ownership_like_cpp() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 5_571);
    let groups = Arc::new(GroupRegistry::default());
    let group = GroupInfo::new(player_guid);
    let group_guid = group.group_guid;
    groups.register_group_like_cpp(group_guid, group);

    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_map_store(canonical_player_transfer_test_map_store_like_cpp());
    session.set_group_registry(Arc::clone(&groups), Arc::new(PendingInvites::default()));
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

    assert!(set_owned_player_group_like_cpp(
        &mut session,
        Some((group_guid, 0))
    ));
    assert!(session.group_reset_update_sequence_for_test());
    assert_eq!(resolved_group_guid_like_cpp(&session), Some(group_guid));
    assert_eq!(session.group_next_update_sequence_for_test(0), Some(1));
    assert!(session.character_remove_current_player_from_canonical_current_map_for_test());
    assert_eq!(
        canonical
            .lock()
            .unwrap()
            .player_residence_like_cpp(old_handle),
        Some(wow_map::PlayerResidenceLikeCpp::Detached)
    );
    assert_eq!(resolved_group_guid_like_cpp(&session), Some(group_guid));
    assert_eq!(session.group_next_update_sequence_for_test(0), Some(2));

    let replacement_state = wow_entities::PlayerGroupState {
        group_guid: ObjectGuid::create_group(group_guid + 1),
        leader_guid: ObjectGuid::create_player(1, 9_999),
        role_mask: 4,
        subgroup: 3,
    };
    let mut replacement = Box::new(Player::new(Some(2), false));
    replacement
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(player_guid);
    replacement.gameplay_state_mut().group = Some(replacement_state.clone());
    let replacement_handle = canonical
        .lock()
        .unwrap()
        .install_detached_player_like_cpp(replacement)
        .expect("replacement owner");

    assert_eq!(resolved_group_guid_like_cpp(&session), None);
    assert!(!set_owned_player_group_like_cpp(&mut session, None));
    assert_eq!(session.group_next_update_sequence_for_test(0), None);
    assert_eq!(
        canonical
            .lock()
            .unwrap()
            .with_player_like_cpp(replacement_handle, |player| {
                player.gameplay_state().group.clone()
            }),
        Some(Some(replacement_state))
    );
}

#[test]
fn player_registry_publishes_home_group_party_type_like_cpp() {
    let (mut session, _, _) = make_session();
    let guid = ObjectGuid::create_player(1, 46);
    let registry = Arc::new(PlayerRegistry::default());
    let canonical = shared_canonical_map_manager();
    assert!(registry.bind_canonical_map_manager(Arc::clone(&canonical)));
    session.set_canonical_map_manager(Arc::clone(&canonical));
    add_canonical_test_player_on_map(&canonical, guid, Position::new(1.0, 2.0, 3.0, 0.0), 571, 0);
    let group_registry = Arc::new(GroupRegistry::default());
    let position = Position::new(1.0, 2.0, 3.0, 0.0);
    let group = GroupInfo::new(guid);
    let group_guid = group.group_guid;
    group_registry.register_group_like_cpp(group_guid, group);
    session.set_player_guid(Some(guid));
    session.character_set_player_map_position_for_test(571, position);
    set_loaded_player_name_like_cpp(&mut session, "PartyTypeTester".to_string());
    set_group_guid_for_test_like_cpp(&mut session, Some(group_guid));
    session.set_player_registry(Arc::clone(&registry));
    session.set_group_registry(group_registry, Arc::new(PendingInvites::default()));

    register_in_player_registry_production_for_test(&session);

    let party_type = canonical_party_type_for_test(&canonical, guid);
    assert_eq!(
        party_type,
        [
            wow_social::group::GROUP_TYPE_NORMAL_LIKE_CPP,
            wow_social::group::GROUP_TYPE_NONE_LIKE_CPP
        ]
    );
}
