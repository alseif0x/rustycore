//! Original Group Session application cases.

use super::*;

#[tokio::test]
async fn apply_group_subgroup_command_updates_current_group_reference_like_cpp() {
    let (mut session, _, _) = make_session();
    let group_guid = 0xABCDEF;
    set_group_guid_for_test_like_cpp(&mut session, Some(group_guid));
    session.set_state(SessionState::LoggedIn);

    session
        .session_command_tx()
        .try_send(SessionCommand::ApplyGroupSubgroupLikeCpp(
            ApplyGroupSubgroupLikeCppCommand {
                group_guid,
                subgroup: 4,
            },
        ))
        .unwrap();
    process_represented_session_commands_like_cpp(&mut session).await;

    assert_eq!(represented_subgroup_like_cpp(&session), Some(4));

    session
        .session_command_tx()
        .try_send(SessionCommand::ApplyGroupSubgroupLikeCpp(
            ApplyGroupSubgroupLikeCppCommand {
                group_guid: group_guid + 1,
                subgroup: 2,
            },
        ))
        .unwrap();
    process_represented_session_commands_like_cpp(&mut session).await;

    assert_eq!(represented_subgroup_like_cpp(&session), Some(4));
}

#[tokio::test]
async fn apply_group_subgroup_command_ignores_non_logged_in_session_like_cpp() {
    let (mut session, _, _) = make_session();
    let group_guid = 0xABCDEF;
    set_group_guid_for_test_like_cpp(&mut session, Some(group_guid));

    session
        .session_command_tx()
        .try_send(SessionCommand::ApplyGroupSubgroupLikeCpp(
            ApplyGroupSubgroupLikeCppCommand {
                group_guid,
                subgroup: 4,
            },
        ))
        .unwrap();
    process_represented_session_commands_like_cpp(&mut session).await;

    assert_eq!(represented_subgroup_like_cpp(&session), None);
}

#[tokio::test]
async fn party_update_command_consumes_receiver_sequence_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    let group_registry = Arc::new(GroupRegistry::default());
    let group = GroupInfo::new(player_guid);
    let group_guid = group.group_guid;
    group_registry.register_group_like_cpp(group_guid, group);
    session.set_group_registry(group_registry, Arc::new(PendingInvites::default()));
    assert!(set_owned_player_group_like_cpp(
        &mut session,
        Some((group_guid, 0))
    ));
    session.set_player_guid(Some(player_guid));
    session.set_state(SessionState::LoggedIn);

    assert!(session.group_reset_update_sequence_for_test());

    for _ in 0..2 {
        session
            .session_command_tx()
            .try_send(SessionCommand::SendPartyUpdateLikeCpp(
                SendPartyUpdateLikeCppCommand {
                    recipient: player_guid,
                    party_update: wow_packet::packets::party::PartyUpdate {
                        party_flags: 0,
                        party_index: wow_social::group::GROUP_CATEGORY_HOME_LIKE_CPP,
                        party_type: wow_social::group::GROUP_TYPE_NORMAL_LIKE_CPP,
                        my_index: 0,
                        party_guid: group_guid,
                        // C++ ignores any caller/global sequence and asks
                        // the receiver Player for the next number.
                        sequence_num: 999,
                        leader_guid: player_guid,
                        leader_faction_group: 0,
                        player_list: Vec::new(),
                        loot_settings: None,
                        difficulty_settings: None,
                    },
                    member_full_state_packets: Vec::new(),
                },
            ))
            .unwrap();
        process_represented_session_commands_like_cpp(&mut session).await;
    }

    let first = send_rx.try_recv().unwrap();
    let second = send_rx.try_recv().unwrap();
    assert_eq!(party_update_sequence_num_like_cpp(&first), 1);
    assert_eq!(party_update_sequence_num_like_cpp(&second), 2);
}

#[tokio::test]
async fn group_removal_command_clears_remote_party_type_like_cpp() {
    let (mut session, _, instance_rx) = make_session();
    let (realm_tx, realm_rx) = flume::bounded(8);
    install_realm_send_channel_for_test(&mut session, realm_tx);
    let player_guid = ObjectGuid::create_player(1, 42);
    let group_guid = 0xABCDEF;
    let player_registry = Arc::new(wow_world::session::directory::PlayerRegistry::default());
    let canonical = shared_canonical_map_manager();
    assert!(player_registry.bind_canonical_map_manager(Arc::clone(&canonical)));
    session.set_canonical_map_manager(Arc::clone(&canonical));
    let (registry_send_tx, _registry_send_rx) = flume::bounded(8);
    let group_registry = Arc::new(GroupRegistry::default());
    let mut group = GroupInfo::new(player_guid);
    group.group_guid = group_guid;
    group_registry.register_group_like_cpp(group_guid, group);

    session.set_player_guid(Some(player_guid));
    set_group_guid_for_test_like_cpp(&mut session, Some(group_guid));
    session.set_state(SessionState::LoggedIn);
    session.character_attach_player_controller_for_test(
        player_guid,
        "Tester".to_string(),
        Position::ZERO,
        571,
        1,
        1,
        80,
        0,
    );
    add_canonical_test_player_on_map(&canonical, player_guid, Position::ZERO, 571, 0);
    session.set_player_registry(Arc::clone(&player_registry));
    session.set_group_registry(
        Arc::clone(&group_registry),
        Arc::new(PendingInvites::default()),
    );
    let mut registration =
        broadcast_info_with_command(player_guid, registry_send_tx, session.session_command_tx());
    registration.placement.map_id = 571;
    player_registry.register_or_replace(player_guid, registration, Default::default());
    sync_player_registry_state_production_for_test(&session);

    let before = canonical_party_type_for_test(&canonical, player_guid);
    assert_eq!(
        before[usize::from(wow_social::group::GROUP_CATEGORY_HOME_LIKE_CPP)],
        wow_social::group::GROUP_TYPE_NORMAL_LIKE_CPP
    );

    session
        .session_command_tx()
        .try_send(SessionCommand::ApplyGroupRemovalLikeCpp(
            ApplyGroupRemovalLikeCppCommand {
                group_guid,
                category: wow_social::group::GROUP_CATEGORY_HOME_LIKE_CPP,
                party_type: wow_social::group::GROUP_TYPE_NONE_LIKE_CPP,
                send_group_destroyed: true,
                send_group_uninvite: false,
                refresh_visible_gameobjects_or_spellclicks: false,
            },
        ))
        .unwrap();
    group_registry.unregister_group_like_cpp(&group_guid);
    process_represented_session_commands_like_cpp(&mut session).await;

    assert_eq!(group_guid_for_test_like_cpp(&session), None);
    let after = canonical_party_type_for_test(&canonical, player_guid);
    assert_eq!(
        after[usize::from(wow_social::group::GROUP_CATEGORY_HOME_LIKE_CPP)],
        wow_social::group::GROUP_TYPE_NONE_LIKE_CPP
    );

    let packets = drain_server_packet_bytes(&instance_rx);
    assert!(packets.iter().any(|bytes| {
        wow_packet::WorldPacket::from_bytes(bytes).server_opcode()
            == Some(ServerOpcodes::UpdateObject)
    }));
    assert!(!packets.iter().any(|bytes| {
        wow_packet::WorldPacket::from_bytes(bytes).server_opcode()
            == Some(ServerOpcodes::GroupDestroyed)
    }));
    let destroyed = realm_rx.try_recv().expect("realm GroupDestroyed");
    assert_eq!(
        wow_packet::WorldPacket::from_bytes(&destroyed).server_opcode(),
        Some(ServerOpcodes::GroupDestroyed)
    );
    // C++ `Group::Disband` sends the destroyed `PartyUpdate` after `GroupDestroyed`.
    let destroyed_update = realm_rx.try_recv().expect("realm destroyed PartyUpdate");
    assert_destroyed_party_update_like_cpp(&destroyed_update, group_guid);
    assert!(realm_rx.try_recv().is_err());
}

#[tokio::test]
async fn group_removal_command_can_send_group_uninvite_like_cpp() {
    let (mut session, _, instance_rx) = make_session();
    let (realm_tx, realm_rx) = flume::bounded(8);
    install_realm_send_channel_for_test(&mut session, realm_tx);
    let player_guid = ObjectGuid::create_player(1, 42);
    let group_guid = 0xBCDEF0;
    let player_registry = Arc::new(wow_world::session::directory::PlayerRegistry::default());
    let (registry_send_tx, _registry_send_rx) = flume::bounded(8);
    let group_registry = Arc::new(GroupRegistry::default());
    let mut group = GroupInfo::new(player_guid);
    group.group_guid = group_guid;
    group_registry.register_group_like_cpp(group_guid, group);

    session.set_player_guid(Some(player_guid));
    set_group_guid_for_test_like_cpp(&mut session, Some(group_guid));
    session.set_state(SessionState::LoggedIn);
    session.character_attach_player_controller_for_test(
        player_guid,
        "Tester".to_string(),
        Position::ZERO,
        571,
        1,
        1,
        80,
        0,
    );
    session.set_player_registry(Arc::clone(&player_registry));
    session.set_group_registry(
        Arc::clone(&group_registry),
        Arc::new(PendingInvites::default()),
    );
    player_registry.register_or_replace(
        player_guid,
        broadcast_info(player_guid, registry_send_tx),
        Default::default(),
    );
    group_registry.unregister_group_like_cpp(&group_guid);

    session
        .session_command_tx()
        .try_send(SessionCommand::ApplyGroupRemovalLikeCpp(
            ApplyGroupRemovalLikeCppCommand {
                group_guid,
                category: wow_social::group::GROUP_CATEGORY_HOME_LIKE_CPP,
                party_type: wow_social::group::GROUP_TYPE_NONE_LIKE_CPP,
                send_group_destroyed: false,
                send_group_uninvite: true,
                refresh_visible_gameobjects_or_spellclicks: false,
            },
        ))
        .unwrap();
    process_represented_session_commands_like_cpp(&mut session).await;

    let packets = drain_server_packet_bytes(&instance_rx);
    assert!(packets.iter().any(|bytes| {
        wow_packet::WorldPacket::from_bytes(bytes).server_opcode()
            == Some(ServerOpcodes::UpdateObject)
    }));
    assert!(!packets.iter().any(|bytes| {
        wow_packet::WorldPacket::from_bytes(bytes).server_opcode()
            == Some(ServerOpcodes::GroupUninvite)
    }));
    assert!(!packets.iter().any(|bytes| {
        wow_packet::WorldPacket::from_bytes(bytes).server_opcode()
            == Some(ServerOpcodes::GroupDestroyed)
    }));
    let uninvite = realm_rx.try_recv().expect("realm GroupUninvite");
    assert_eq!(
        wow_packet::WorldPacket::from_bytes(&uninvite).server_opcode(),
        Some(ServerOpcodes::GroupUninvite)
    );
    // C++ `Group::RemoveMember` sends the kicked member the destroyed
    // `PartyUpdate` when the group survives (`Group.cpp:654-655`).
    let destroyed_update = realm_rx.try_recv().expect("realm destroyed PartyUpdate");
    assert_destroyed_party_update_like_cpp(&destroyed_update, group_guid);
    assert!(realm_rx.try_recv().is_err());
}
