//! Group scenarios for [`super`].
//!
//! Split out of group_tests.rs under #628; assertions and
//! registrations are unchanged and shared fixtures stay in the parent module.

use super::*;

#[tokio::test]
async fn party_invite_low_level_friend_port_preserves_ignore_then_friend_order_like_cpp() {
    let (mut session, _send_rx) = make_session_with_send();
    let inviter = ObjectGuid::create_player(1, 42);
    let target = ObjectGuid::create_player(1, 77);
    let target_name = format!("Player{}", target.low_value());

    let player_registry = Arc::new(PlayerRegistry::with_canonical_player_fixtures_like_cpp());
    let (target_tx, target_rx) = bounded(8);
    player_registry.register_or_replace(
        target,
        broadcast_info(target, target_tx),
        Default::default(),
    );
    let pending_invites = Arc::new(PendingInvites::default());
    let port = PartyInviteSocialPortLikeCpp::new(
        SocialPartyInviteLookupOutcomeLikeCpp::Resolved(false),
        SocialPartyInviteLookupOutcomeLikeCpp::Resolved(true),
    );

    session.set_player_guid(Some(inviter));
    session.set_loaded_player_identity_like_cpp(0, 1, 1, 1, 0);
    session.set_party_level_req_like_cpp(2);
    session.set_player_registry(player_registry);
    session.set_group_registry(
        Arc::new(GroupRegistry::default()),
        Arc::clone(&pending_invites),
    );
    session.set_social_persistence_port_like_cpp(port.clone());

    session
        .handle_party_invite(party_invite_packet(target, &target_name, None, 0))
        .await;

    assert_eq!(port.calls(), vec!["ignore:77:42:1", "friend:77:42"]);
    assert!(pending_invites.get(&target).is_some());
    assert!(party_invite_can_accept(&recv_dispatched_packet(
        &target_rx,
        "target invite packet"
    )));
}
#[tokio::test]
async fn failed_party_invite_social_lookup_retains_the_existing_fail_open_result() {
    let inviter = ObjectGuid::create_player(1, 42);
    let target = ObjectGuid::create_player(1, 77);
    let port = PartyInviteSocialPortLikeCpp::new(
        SocialPartyInviteLookupOutcomeLikeCpp::Failed {
            reason: "database unavailable".to_owned(),
        },
        SocialPartyInviteLookupOutcomeLikeCpp::Resolved(false),
    );

    assert!(
        !super::super::target_social_ignores_inviter_like_cpp(
            Some(port.clone()),
            target,
            inviter,
            1,
        )
        .await
    );
    assert_eq!(port.calls(), vec!["ignore:77:42:1"]);
}
#[tokio::test]
async fn party_invite_rejects_same_map_different_instances_like_cpp() {
    let (mut session, send_rx) = make_session_with_send();
    let inviter = ObjectGuid::create_player(1, 42);
    let target = ObjectGuid::create_player(1, 77);
    let target_name = format!("Player{}", target.low_value());

    let player_registry = Arc::new(PlayerRegistry::with_canonical_player_fixtures_like_cpp());
    let (inviter_tx, _inviter_rx) = bounded(8);
    let mut inviter_info = broadcast_info(inviter, inviter_tx);
    inviter_info.placement.map_id = 571;
    inviter_info.placement.instance_id = 100;
    player_registry.register_or_replace(inviter, inviter_info, Default::default());
    let (target_tx, target_rx) = bounded(8);
    let mut target_info = broadcast_info(target, target_tx);
    target_info.placement.map_id = 571;
    target_info.placement.instance_id = 200;
    player_registry.register_or_replace(target, target_info, Default::default());
    let pending_invites = Arc::new(PendingInvites::default());

    session.set_player_guid(Some(inviter));
    session.set_loaded_player_identity_like_cpp(571, 1, 1, 80, 0);
    session.set_player_registry(player_registry);
    session.set_group_registry(
        Arc::new(GroupRegistry::default()),
        Arc::clone(&pending_invites),
    );

    session
        .handle_party_invite(party_invite_packet(target, &target_name, None, 0))
        .await;

    assert_eq!(
        party_command_result_code(&send_rx.try_recv().expect("party command result")),
        party_result::TARGET_NOT_IN_INSTANCE
    );
    assert!(target_rx.try_recv().is_err());
    assert!(pending_invites.get(&target).is_none());
}
#[tokio::test]
async fn party_invite_rejects_instance_difficulty_mismatch_like_cpp() {
    let (mut session, send_rx) = make_session_with_send();
    let inviter = ObjectGuid::create_player(1, 42);
    let target = ObjectGuid::create_player(1, 77);
    let target_name = format!("Player{}", target.low_value());

    let player_registry = Arc::new(PlayerRegistry::default());
    let (target_tx, target_rx) = bounded(8);
    let mut target_info = broadcast_info(target, target_tx);
    target_info.placement.map_id = 571;
    target_info.placement.instance_id = 100;
    player_registry.register_or_replace(target, target_info, Default::default());
    let canonical = Arc::new(std::sync::Mutex::new(wow_map::MapManager::default()));
    let mut target_player = wow_entities::Player::new(Some(1), false);
    target_player
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(target);
    target_player
        .unit_mut()
        .world_mut()
        .set_map(571, 100)
        .unwrap();
    target_player
        .unit_mut()
        .world_mut()
        .object_mut()
        .add_to_world();
    target_player.gameplay_state_mut().dungeon_difficulty_id = 2;
    canonical
        .lock()
        .unwrap()
        .create_world_map(571, 100)
        .map_mut()
        .insert_map_object_record(wow_entities::MapObjectRecord::new_player(target_player).unwrap())
        .unwrap();
    assert!(player_registry.bind_canonical_map_manager(canonical));
    let pending_invites = Arc::new(PendingInvites::default());

    session.set_player_guid(Some(inviter));
    session.set_loaded_player_identity_like_cpp(571, 1, 1, 80, 0);
    session.set_represented_dungeon_difficulty_id_for_test_like_cpp(1);
    session.set_player_registry(player_registry);
    session.set_group_registry(
        Arc::new(GroupRegistry::default()),
        Arc::clone(&pending_invites),
    );

    session
        .handle_party_invite(party_invite_packet(target, &target_name, None, 0))
        .await;

    assert_eq!(
        party_command_result_code(&send_rx.try_recv().expect("party command result")),
        party_result::IGNORING_YOU
    );
    assert!(target_rx.try_recv().is_err());
    assert!(pending_invites.get(&target).is_none());
}
#[tokio::test]
async fn party_invite_response_party_index_mismatch_keeps_invite_pending_like_cpp() {
    let (mut session, send_rx) = make_session_with_send();
    let inviter = ObjectGuid::create_player(1, 42);
    let target = ObjectGuid::create_player(1, 77);

    let player_registry = Arc::new(PlayerRegistry::with_canonical_player_fixtures_like_cpp());
    let group_registry = Arc::new(GroupRegistry::default());
    let home_group = GroupInfo::new(inviter);
    let home_group_guid = home_group.group_guid;
    group_registry.register_group_like_cpp(home_group_guid, home_group);

    let pending_invites = Arc::new(PendingInvites::default());
    pending_invites.seed_invite_like_cpp(
        target,
        PendingInviteLikeCpp::new_existing_group(
            inviter,
            home_group_guid,
            GROUP_CATEGORY_HOME_LIKE_CPP,
        ),
    );

    session.set_player_guid(Some(target));
    session.set_player_registry(player_registry);
    session.set_group_registry(Arc::clone(&group_registry), Arc::clone(&pending_invites));

    session
        .handle_party_invite_response(party_invite_response_packet(
            true,
            Some(wow_social::group::GROUP_CATEGORY_INSTANCE_LIKE_CPP),
            None,
        ))
        .await;

    assert!(
        pending_invites.get(&target).is_some(),
        "C++ checks group category before removing the invite"
    );
    assert!(
        !group_registry
            .get(&home_group_guid)
            .unwrap()
            .members
            .contains(&target),
        "PartyIndex INSTANCE must not add the invitee to the HOME group"
    );
    assert!(send_rx.try_recv().is_err());
    assert!(session.group_guid.is_none());
}
#[tokio::test]
async fn party_invite_response_reports_group_full_without_adding_member_like_cpp() {
    let (mut session, send_rx) = make_session_with_send();
    let leader = ObjectGuid::create_player(1, 42);
    let target = ObjectGuid::create_player(1, 77);

    let group_registry = Arc::new(GroupRegistry::default());
    let mut group = GroupInfo::new(leader);
    for counter in 43..47 {
        assert!(group.add_member(ObjectGuid::create_player(1, counter)));
    }
    let group_guid = group.group_guid;
    group_registry.register_group_like_cpp(group_guid, group);

    let pending_invites = Arc::new(PendingInvites::default());
    pending_invites.seed_invite_like_cpp(
        target,
        PendingInviteLikeCpp::new_existing_group(leader, group_guid, GROUP_CATEGORY_HOME_LIKE_CPP),
    );

    session.set_player_guid(Some(target));
    session.set_player_registry(Arc::new(
        PlayerRegistry::with_canonical_player_fixtures_like_cpp(),
    ));
    session.set_group_registry(Arc::clone(&group_registry), Arc::clone(&pending_invites));

    session
        .handle_party_invite_response(party_invite_response_packet(true, None, None))
        .await;

    assert!(
        pending_invites.get(&target).is_none(),
        "C++ removes the invite before its full-group check"
    );
    assert_eq!(
        party_command_result_code(&send_rx.try_recv().expect("party command result")),
        party_result::GROUP_FULL
    );
    let group = group_registry
        .get(&group_guid)
        .expect("group remains registered");
    assert_eq!(
        group.members.len(),
        wow_social::group::MAX_GROUP_SIZE_LIKE_CPP
    );
    assert!(!group.members.contains(&target));
    assert!(session.group_guid.is_none());
}
#[tokio::test]
async fn party_invite_response_add_member_failure_returns_silently_like_cpp() {
    let (mut session, send_rx) = make_session_with_send();
    let leader = ObjectGuid::create_player(1, 42);
    let target = ObjectGuid::create_player(1, 77);

    let group_registry = Arc::new(GroupRegistry::default());
    let mut group = GroupInfo::new(leader);
    group.convert_to_raid_like_cpp();
    group.raid_subgroup_counts = Some(
        [wow_social::group::MAX_GROUP_SIZE_LIKE_CPP as u8;
            wow_social::group::MAX_RAID_SUBGROUPS_LIKE_CPP],
    );
    let group_guid = group.group_guid;
    group_registry.register_group_like_cpp(group_guid, group);

    let pending_invites = Arc::new(PendingInvites::default());
    pending_invites.seed_invite_like_cpp(
        target,
        PendingInviteLikeCpp::new_existing_group(leader, group_guid, GROUP_CATEGORY_HOME_LIKE_CPP),
    );

    session.set_player_guid(Some(target));
    session.set_player_registry(Arc::new(
        PlayerRegistry::with_canonical_player_fixtures_like_cpp(),
    ));
    session.set_group_registry(Arc::clone(&group_registry), Arc::clone(&pending_invites));

    session
        .handle_party_invite_response(party_invite_response_packet(true, None, None))
        .await;

    assert!(pending_invites.get(&target).is_none());
    assert!(
        send_rx.try_recv().is_err(),
        "C++ sends no GROUP_FULL packet when AddMember itself returns false"
    );
    let group = group_registry
        .get(&group_guid)
        .expect("group remains registered");
    assert_eq!(group.members, vec![leader]);
    assert!(!group.members.contains(&target));
    assert!(session.group_guid.is_none());
}
#[tokio::test]
async fn leave_group_party_index_instance_does_not_leave_home_group_like_cpp() {
    let (mut session, send_rx) = make_session_with_send();
    let leaving_guid = ObjectGuid::create_player(1, 42);
    let other_guid = ObjectGuid::create_player(1, 77);

    let player_registry = Arc::new(PlayerRegistry::with_canonical_player_fixtures_like_cpp());
    let group_registry = Arc::new(GroupRegistry::default());
    let mut home_group = GroupInfo::new(leaving_guid);
    home_group.add_member(other_guid);
    let home_group_guid = home_group.group_guid;
    group_registry.register_group_like_cpp(home_group_guid, home_group);

    session.set_player_guid(Some(leaving_guid));
    session.group_guid = Some(home_group_guid);
    session.set_player_registry(player_registry);
    session.set_group_registry(group_registry.clone(), Arc::new(PendingInvites::default()));

    session
        .handle_leave_group(leave_group_packet(Some(
            wow_social::group::GROUP_CATEGORY_INSTANCE_LIKE_CPP,
        )))
        .await;

    assert!(
        group_registry
            .get(&home_group_guid)
            .unwrap()
            .members
            .contains(&leaving_guid),
        "PartyIndex INSTANCE must not resolve and leave the HOME group"
    );
    assert_eq!(session.group_guid, Some(home_group_guid));
    assert!(send_rx.try_recv().is_err());
}
#[tokio::test]
async fn normal_group_uninvite_in_battleground_returns_invite_restricted_like_cpp() {
    // C++ `CanUninviteFromGroup` normal branch returns
    // `ERR_INVITE_RESTRICTED` when the sender is in a battleground
    // (`Player.cpp:25181-25182`).
    let leader = ObjectGuid::create_player(1, 42);
    let target = ObjectGuid::create_player(1, 100);
    let mut group = GroupInfo::new(leader);
    assert!(group.add_member(target));
    let (mut session, send_rx, group_registry, group_guid) =
        lfg_uninvite_session_like_cpp(group, leader);
    session.set_player_battleground_type_id_like_cpp(1);

    session
        .handle_party_uninvite(party_uninvite_packet(target, None, "bye"))
        .await;

    assert_eq!(
        party_command_result_code(&send_rx.try_recv().expect("invite restricted result")),
        party_result::INVITE_RESTRICTED
    );
    assert!(
        group_registry
            .get(&group_guid)
            .expect("group")
            .members
            .contains(&target)
    );
}
#[tokio::test]
async fn party_uninvite_disband_sends_destroyed_party_update_like_cpp() {
    let (mut session, send_rx) = make_session_with_send();
    let leader = ObjectGuid::create_player(1, 42);
    let target = ObjectGuid::create_player(1, 77);
    let (leader_tx, _leader_rx) = bounded(8);
    let (target_tx, _target_rx) = bounded(8);
    let (target_command_tx, target_command_rx) = bounded(8);
    let player_registry = Arc::new(PlayerRegistry::with_canonical_player_fixtures_like_cpp());
    player_registry.register_or_replace(
        leader,
        broadcast_info(leader, leader_tx),
        Default::default(),
    );
    player_registry.register_or_replace(
        target,
        broadcast_info_with_command_tx(target, target_tx, target_command_tx),
        Default::default(),
    );
    let group_registry = Arc::new(GroupRegistry::default());
    let mut group = GroupInfo::new(leader);
    group.add_member(target);
    let group_guid = group.group_guid;
    group_registry.register_group_like_cpp(group_guid, group);

    session.set_player_guid(Some(leader));
    session.group_guid = Some(group_guid);
    session.set_player_registry(Arc::clone(&player_registry));
    session.set_group_registry(
        Arc::clone(&group_registry),
        Arc::new(PendingInvites::default()),
    );

    session
        .handle_party_uninvite(party_uninvite_packet(target, None, "bye"))
        .await;

    // C++ `Group::RemoveMember` disbands a two-member group instead of
    // keeping it alive (`Group.cpp:660-663`).
    assert!(group_registry.get(&group_guid).is_none());
    assert_eq!(session.group_guid, None);

    let command = target_command_rx.try_recv().unwrap();
    let SessionCommand::ApplyGroupRemovalLikeCpp(command) = command else {
        panic!("expected ApplyGroupRemovalLikeCpp for kicked member");
    };
    assert!(command.send_group_destroyed);
    assert!(!command.send_group_uninvite);

    // C++ `Group::Disband` sends the leader `GroupDestroyed` and then the
    // destroyed `PartyUpdate` (`Group.cpp:744-746`).
    let mut destroyed_index = None;
    let mut update_index = None;
    let mut destroyed_update = None;
    let mut index = 0usize;
    while let Ok(bytes) = send_rx.try_recv() {
        let opcode = WorldPacket::from_bytes(&bytes).server_opcode();
        if opcode == Some(ServerOpcodes::GroupDestroyed) {
            destroyed_index = Some(index);
        }
        if opcode == Some(ServerOpcodes::PartyUpdate) {
            update_index = Some(index);
            destroyed_update = Some(bytes);
        }
        index += 1;
    }
    let destroyed_index = destroyed_index.expect("leader GroupDestroyed");
    let update_index = update_index.expect("leader destroyed PartyUpdate");
    assert!(destroyed_index < update_index);

    let mut packet = WorldPacket::from_bytes(&destroyed_update.unwrap());
    assert_eq!(
        packet.read_uint16().expect("opcode"),
        ServerOpcodes::PartyUpdate as u16
    );
    assert_eq!(
        packet.read_uint16().expect("party flags"),
        wow_social::group::GROUP_FLAG_DESTROYED_LIKE_CPP
    );
    assert_eq!(
        packet.read_uint8().expect("party index"),
        GROUP_CATEGORY_HOME_LIKE_CPP
    );
    assert_eq!(
        packet.read_uint8().expect("party type"),
        wow_social::group::GROUP_TYPE_NONE_LIKE_CPP
    );
    assert_eq!(packet.read_int32().expect("my index"), -1);
    assert_eq!(
        packet.read_packed_guid().expect("party guid"),
        ObjectGuid::create_group(group_guid)
    );
}
#[tokio::test]
async fn party_uninvite_non_leader_rejects_with_cpp_result() {
    let (mut session, send_rx) = make_session_with_send();
    let leader = ObjectGuid::create_player(1, 42);
    let sender = ObjectGuid::create_player(1, 77);
    let target = ObjectGuid::create_player(1, 88);
    let player_registry = Arc::new(PlayerRegistry::with_canonical_player_fixtures_like_cpp());
    let group_registry = Arc::new(GroupRegistry::default());
    let mut group = GroupInfo::new(leader);
    group.add_member(sender);
    group.add_member(target);
    let group_guid = group.group_guid;
    group_registry.register_group_like_cpp(group_guid, group);

    session.set_player_guid(Some(sender));
    session.group_guid = Some(group_guid);
    session.set_player_registry(player_registry);
    session.set_group_registry(group_registry, Arc::new(PendingInvites::default()));

    session
        .handle_party_uninvite(party_uninvite_packet(target, None, "bye"))
        .await;

    let result = send_rx.try_recv().expect("party command result");
    assert_eq!(
        u16::from_le_bytes([result[0], result[1]]),
        ServerOpcodes::PartyCommandResult as u16
    );
    let mut payload = WorldPacket::from_bytes(&result[2..]);
    let name_len = payload.read_bits(9).unwrap();
    let command = payload.read_bits(4).unwrap();
    let result_code = payload.read_bits(6).unwrap();

    assert_eq!(name_len, 0);
    assert_eq!(command, 1); // C++ PARTY_OP_UNINVITE
    assert_eq!(result_code as u8, party_result::NOT_LEADER);
    payload.flush_bits();
    assert_eq!(payload.read_uint32().unwrap(), 0); // C++ ResultData
    // C++ `WorldSession::SendPartyResult` always leaves `ResultGUID`
    // empty (`GroupHandler.cpp:53`).
    assert_eq!(payload.read_packed_guid().unwrap(), ObjectGuid::EMPTY);
    assert!(send_rx.try_recv().is_err());
}
#[tokio::test]
async fn party_uninvite_removes_pending_group_invite_like_cpp() {
    let (mut session, send_rx) = make_session_with_send();
    let leader = ObjectGuid::create_player(1, 42);
    let inviter = ObjectGuid::create_player(1, 77);
    let target = ObjectGuid::create_player(1, 88);
    let player_registry = Arc::new(PlayerRegistry::with_canonical_player_fixtures_like_cpp());
    let group_registry = Arc::new(GroupRegistry::default());
    let pending_invites = Arc::new(PendingInvites::default());
    let mut group = GroupInfo::new(leader);
    group.add_member(inviter);
    let group_guid = group.group_guid;
    group_registry.register_group_like_cpp(group_guid, group);
    pending_invites.seed_invite_like_cpp(
        target,
        PendingInviteLikeCpp::new_existing_group(leader, group_guid, GROUP_CATEGORY_HOME_LIKE_CPP),
    );

    session.set_player_guid(Some(leader));
    session.group_guid = Some(group_guid);
    session.set_player_registry(player_registry);
    session.set_group_registry(Arc::clone(&group_registry), Arc::clone(&pending_invites));

    session
        .handle_party_uninvite(party_uninvite_packet(target, None, "revoked"))
        .await;

    assert!(
        pending_invites.get(&target).is_none(),
        "C++ Player::UninviteFromGroup clears the pending group invite"
    );
    let group = group_registry.get(&group_guid).unwrap();
    assert!(group.members.contains(&leader));
    assert!(group.members.contains(&inviter));
    assert!(!group.members.contains(&target));
    drop(group);
    assert!(
        send_rx.try_recv().is_err(),
        "pending invite removal returns without ERR_TARGET_NOT_IN_GROUP_S"
    );
}
#[test]
fn party_member_full_state_carries_phase_states_like_cpp() {
    let leader = ObjectGuid::create_player(1, 42);
    let member = ObjectGuid::create_player(1, 77);
    let (leader_tx, leader_rx) = bounded(8);
    let (member_tx, _member_rx) = bounded(8);
    let registry = PlayerRegistry::with_canonical_player_fixtures_like_cpp();
    registry.register_or_replace(
        leader,
        broadcast_info(leader, leader_tx),
        Default::default(),
    );
    registry.register_or_replace(
        member,
        broadcast_info(member, member_tx),
        Default::default(),
    );
    let canonical = bind_canonical_party_players_like_cpp(&registry, [leader, member]);
    assert!(
        crate::canonical_player_access::with_canonical_player_at_mut_like_cpp(
            &canonical,
            member,
            0,
            0,
            |player| {
                let phase = player.unit_mut().world_mut().phase_shift_mut();
                phase.add_phase_like_cpp(20, wow_constants::PhaseFlags::PERSONAL, 1);
                phase.set_flags_like_cpp(wow_constants::PhaseShiftFlags::UNPHASED);
            },
        )
        .is_some()
    );
    let mut group = GroupInfo::new(leader);
    group.members.push(member);

    send_party_update(&group, &registry, 0);

    let _party_update = recv_dispatched_packet(&leader_rx, "leader PartyUpdate");
    let full_state = recv_dispatched_packet(&leader_rx, "leader PartyMemberFullState");
    assert_eq!(
        u16::from_le_bytes([full_state[0], full_state[1]]),
        ServerOpcodes::PartyMemberFullState as u16
    );
    let phase_bytes = [
        0x08, 0x00, 0x00, 0x00, // PhaseShiftFlags
        0x01, 0x00, 0x00, 0x00, // List.Count
        0x00, 0x00, // PersonalGUID packed mask + empty payload
        0x02, 0x00, 0x00, 0x00, // phase.Flags
        0x14, 0x00, // phase.Id
    ];
    assert!(
        full_state
            .windows(phase_bytes.len())
            .any(|window| window == phase_bytes)
    );
}
#[test]
fn set_party_leader_dispatch_metadata_matches_cpp() {
    let entry = inventory::iter::<PacketHandlerEntry>
        .into_iter()
        .find(|entry| entry.opcode == ClientOpcodes::SetPartyLeader)
        .expect("SetPartyLeader handler entry");

    assert_eq!(entry.status, SessionStatus::LoggedIn);
    assert_eq!(entry.processing, PacketProcessing::Inplace);
    assert_eq!(entry.handler_name, "handle_set_party_leader");
}
#[test]
fn group_party_update_member_info_uses_loaded_member_slot_like_cpp() {
    let leader = ObjectGuid::create_player(1, 42);
    let member = ObjectGuid::create_player(1, 77);
    let mut group = GroupInfo::loaded_from_db_like_cpp(
        900,
        17,
        leader,
        5,
        leader,
        2,
        0,
        1,
        14,
        3,
        ObjectGuid::EMPTY,
    );
    assert!(group.load_member_from_db_like_cpp(
        77,
        0x04,
        3,
        2,
        Some(GroupMemberCharacterLikeCpp {
            name: "LoadedMember".to_string(),
            race: 8,
            class: 9,
        }),
    ));

    let registry = PlayerRegistry::with_canonical_player_fixtures_like_cpp();
    let (tx, _rx) = bounded(1);
    let mut registration = broadcast_info(member, tx);
    registration.identity.player_name.clear();
    registration.identity.race = 0;
    registration.identity.class = 0;
    registry.register_or_replace(member, registration, Default::default());

    let info = party_player_info_like_cpp(&group, &registry, member)
        .expect("connected represented member should produce party info");
    assert_eq!(info.name, "LoadedMember");
    assert_eq!(info.class, 9);
    assert_eq!(info.subgroup, 3);
    assert_eq!(info.flags, 0x04);
    assert_eq!(info.roles_assigned, 2);
    assert_eq!(info.faction_group, 2);
}
#[tokio::test]
async fn raid_target_symbol_out_of_range_does_not_mutate_or_fanout_like_cpp() {
    let (mut session, _send_rx) = make_session_with_send();
    let leader = ObjectGuid::create_player(1, 42);
    let target = ObjectGuid::create_player(1, 77);
    let group_registry = Arc::new(GroupRegistry::default());
    let group = GroupInfo::new(leader);
    let original_icons = group.target_icons;
    let group_guid = group.group_guid;
    group_registry.register_group_like_cpp(group_guid, group);
    let player_registry = Arc::new(PlayerRegistry::with_canonical_player_fixtures_like_cpp());
    let (leader_tx, leader_rx) = bounded(8);
    let (target_tx, _target_rx) = bounded(8);
    player_registry.register_or_replace(
        leader,
        broadcast_info(leader, leader_tx),
        Default::default(),
    );
    player_registry.register_or_replace(
        target,
        broadcast_info(target, target_tx),
        Default::default(),
    );

    session.set_player_guid(Some(leader));
    session.group_guid = Some(group_guid);
    session.set_player_registry(player_registry);
    session.set_group_registry(group_registry.clone(), Arc::new(PendingInvites::default()));

    session
        .handle_update_raid_target(update_raid_target_packet(target, 8, None))
        .await;

    assert_eq!(
        group_registry.get(&group_guid).unwrap().target_icons,
        original_icons
    );
    assert!(leader_rx.try_recv().is_err());
}
#[tokio::test]
async fn raid_target_non_raid_regular_member_can_set_icon_like_cpp() {
    let (mut session, _send_rx) = make_session_with_send();
    let leader = ObjectGuid::create_player(1, 42);
    let member = ObjectGuid::create_player(1, 43);
    let target = ObjectGuid::create_player(1, 77);
    let group_registry = Arc::new(GroupRegistry::default());
    let mut group = GroupInfo::new(leader);
    group.add_member(member);
    let group_guid = group.group_guid;
    group_registry.register_group_like_cpp(group_guid, group);
    let player_registry = Arc::new(PlayerRegistry::with_canonical_player_fixtures_like_cpp());
    let (leader_tx, leader_rx) = bounded(8);
    let (member_tx, member_rx) = bounded(8);
    let (target_tx, _target_rx) = bounded(8);
    player_registry.register_or_replace(
        leader,
        broadcast_info(leader, leader_tx),
        Default::default(),
    );
    player_registry.register_or_replace(
        member,
        broadcast_info(member, member_tx),
        Default::default(),
    );
    player_registry.register_or_replace(
        target,
        broadcast_info(target, target_tx),
        Default::default(),
    );

    session.set_player_guid(Some(member));
    session.group_guid = Some(group_guid);
    session.set_player_registry(player_registry);
    session.set_group_registry(group_registry.clone(), Arc::new(PendingInvites::default()));

    session
        .handle_update_raid_target(update_raid_target_packet(target, 3, None))
        .await;

    assert_eq!(
        group_registry.get(&group_guid).unwrap().target_icons[3],
        target.to_raw_bytes()
    );
    let leader_sent = leader_rx.try_recv().expect("leader raid target fanout");
    let member_sent = member_rx.try_recv().expect("member raid target fanout");
    assert_eq!(leader_sent, member_sent);
    let mut pkt = WorldPacket::from_bytes(&leader_sent);
    assert_eq!(
        pkt.read_uint16().unwrap(),
        ServerOpcodes::SendRaidTargetUpdateSingle as u16
    );
    assert_eq!(pkt.read_uint8().unwrap(), 0);
    assert_eq!(pkt.read_uint8().unwrap(), 3);
    assert_eq!(pkt.read_packed_guid().unwrap(), target);
    assert_eq!(pkt.read_packed_guid().unwrap(), member);
}
#[tokio::test]
async fn raid_target_raid_regular_member_rejected_but_assistant_allowed_like_cpp() {
    let (mut session, _send_rx) = make_session_with_send();
    let leader = ObjectGuid::create_player(1, 42);
    let assistant = ObjectGuid::create_player(1, 43);
    let target = ObjectGuid::create_player(1, 77);
    let group_registry = Arc::new(GroupRegistry::default());
    let mut group = GroupInfo::new(leader);
    group.add_member(assistant);
    group.convert_to_raid_like_cpp();
    let group_guid = group.group_guid;
    group_registry.register_group_like_cpp(group_guid, group);
    let player_registry = Arc::new(PlayerRegistry::with_canonical_player_fixtures_like_cpp());
    let (leader_tx, leader_rx) = bounded(8);
    let (assistant_tx, assistant_rx) = bounded(8);
    let (target_tx, _target_rx) = bounded(8);
    player_registry.register_or_replace(
        leader,
        broadcast_info(leader, leader_tx),
        Default::default(),
    );
    player_registry.register_or_replace(
        assistant,
        broadcast_info(assistant, assistant_tx),
        Default::default(),
    );
    player_registry.register_or_replace(
        target,
        broadcast_info(target, target_tx),
        Default::default(),
    );

    session.set_player_guid(Some(assistant));
    session.group_guid = Some(group_guid);
    session.set_player_registry(Arc::clone(&player_registry));
    session.set_group_registry(group_registry.clone(), Arc::new(PendingInvites::default()));

    session
        .handle_update_raid_target(update_raid_target_packet(target, 4, None))
        .await;
    assert_eq!(
        group_registry.get(&group_guid).unwrap().target_icons[4],
        wow_social::group::EMPTY_TARGET_ICON_RAW_LIKE_CPP
    );
    assert!(leader_rx.try_recv().is_err());
    assert!(assistant_rx.try_recv().is_err());

    group_registry
        .set_member_flag_transition_like_cpp(
            group_guid,
            leader,
            assistant,
            true,
            wow_social::group::MEMBER_FLAG_ASSISTANT_LIKE_CPP,
        )
        .unwrap();
    session
        .handle_update_raid_target(update_raid_target_packet(target, 4, None))
        .await;
    assert_eq!(
        group_registry.get(&group_guid).unwrap().target_icons[4],
        target.to_raw_bytes()
    );
    assert!(leader_rx.try_recv().is_ok());
    assert!(assistant_rx.try_recv().is_ok());
}
#[tokio::test]
async fn raid_target_duplicate_target_clears_old_icon_before_final_update_like_cpp() {
    let (mut session, _send_rx) = make_session_with_send();
    let leader = ObjectGuid::create_player(1, 42);
    let target = ObjectGuid::create_player(1, 77);
    let group_registry = Arc::new(GroupRegistry::default());
    let mut group = GroupInfo::new(leader);
    group.convert_to_raid_like_cpp();
    group.target_icons[1] = target.to_raw_bytes();
    let group_guid = group.group_guid;
    group_registry.register_group_like_cpp(group_guid, group);
    let player_registry = Arc::new(PlayerRegistry::with_canonical_player_fixtures_like_cpp());
    let (leader_tx, leader_rx) = bounded(8);
    let (target_tx, _target_rx) = bounded(8);
    player_registry.register_or_replace(
        leader,
        broadcast_info(leader, leader_tx),
        Default::default(),
    );
    player_registry.register_or_replace(
        target,
        broadcast_info(target, target_tx),
        Default::default(),
    );

    session.set_player_guid(Some(leader));
    session.group_guid = Some(group_guid);
    session.set_player_registry(player_registry);
    session.set_group_registry(group_registry.clone(), Arc::new(PendingInvites::default()));

    session
        .handle_update_raid_target(update_raid_target_packet(target, 5, None))
        .await;

    let first = leader_rx.try_recv().expect("clear old icon update");
    let mut first_pkt = WorldPacket::from_bytes(&first);
    assert_eq!(
        first_pkt.read_uint16().unwrap(),
        ServerOpcodes::SendRaidTargetUpdateSingle as u16
    );
    assert_eq!(first_pkt.read_uint8().unwrap(), 0);
    assert_eq!(first_pkt.read_uint8().unwrap(), 1);
    assert_eq!(first_pkt.read_packed_guid().unwrap(), ObjectGuid::EMPTY);
    assert_eq!(first_pkt.read_packed_guid().unwrap(), leader);
    let second = leader_rx.try_recv().expect("set new icon update");
    let mut second_pkt = WorldPacket::from_bytes(&second);
    assert_eq!(
        second_pkt.read_uint16().unwrap(),
        ServerOpcodes::SendRaidTargetUpdateSingle as u16
    );
    assert_eq!(second_pkt.read_uint8().unwrap(), 0);
    assert_eq!(second_pkt.read_uint8().unwrap(), 5);
    assert_eq!(second_pkt.read_packed_guid().unwrap(), target);
    let group = group_registry.get(&group_guid).unwrap();
    assert_eq!(
        group.target_icons[1],
        wow_social::group::EMPTY_TARGET_ICON_RAW_LIKE_CPP
    );
    assert_eq!(group.target_icons[5], target.to_raw_bytes());
}
