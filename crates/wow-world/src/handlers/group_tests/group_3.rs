//! Group scenarios for [`super`].
//!
//! Split out of group_tests.rs under #628; assertions and
//! registrations are unchanged and shared fixtures stay in the parent module.

use super::*;

#[tokio::test]
async fn raid_target_party_index_instance_does_not_fall_back_to_home_like_cpp() {
    let (mut session, _send_rx) = make_session_with_send();
    let leader = ObjectGuid::create_player(1, 42);
    let target = ObjectGuid::create_player(1, 77);
    let group_registry = Arc::new(GroupRegistry::default());
    let group = GroupInfo::new(leader);
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
        .handle_update_raid_target(update_raid_target_packet(target, 2, Some(1)))
        .await;

    assert_eq!(
        group_registry.get(&group_guid).unwrap().target_icons[2],
        wow_social::group::EMPTY_TARGET_ICON_RAW_LIKE_CPP
    );
    assert!(leader_rx.try_recv().is_err());
}
#[tokio::test]
async fn party_join_updates_sends_target_list_and_raid_markers_like_cpp() {
    let (mut session, send_rx) = make_session_with_send();
    let leader = ObjectGuid::create_player(1, 42);
    let marked = ObjectGuid::create_player(1, 77);
    let marker_position = Position::xyz(12.25, -34.5, 6.75);
    let group_registry = Arc::new(GroupRegistry::default());
    let mut group = GroupInfo::new(leader);
    group.target_icons[6] = marked.to_raw_bytes();
    group.add_raid_marker_like_cpp(3, 571, marker_position, ObjectGuid::EMPTY);
    let group_guid = group.group_guid;
    group_registry.register_group_like_cpp(group_guid, group);

    session.set_player_guid(Some(leader));
    session.group_guid = Some(group_guid);
    session.set_group_registry(group_registry, Arc::new(PendingInvites::default()));

    session
        .handle_request_party_join_updates(request_party_join_updates_packet(Some(0)))
        .await;

    let target_list = send_rx.try_recv().expect("target list");
    let mut pkt = WorldPacket::from_bytes(&target_list);
    assert_eq!(
        pkt.read_uint16().unwrap(),
        ServerOpcodes::SendRaidTargetUpdateAll as u16
    );
    assert_eq!(pkt.read_uint8().unwrap(), 0);
    assert_eq!(pkt.read_uint32().unwrap(), 8);
    for symbol in 0..8 {
        let target = pkt.read_packed_guid().unwrap();
        assert_eq!(pkt.read_uint8().unwrap(), symbol);
        if symbol == 6 {
            assert_eq!(target, marked);
        }
    }
    let markers = send_rx.try_recv().expect("raid markers");
    assert_raid_markers_packet_like_cpp(&markers, 1 << 3, &[marker_position]);
    assert!(send_rx.try_recv().is_err());
}
#[tokio::test]
async fn clear_raid_marker_removes_one_slot_and_fanouts_like_cpp() {
    let (mut session, _send_rx) = make_session_with_send();
    let leader = ObjectGuid::create_player(1, 42);
    let member = ObjectGuid::create_player(1, 43);
    let remaining_position = Position::xyz(4.0, 5.0, 6.0);
    let group_registry = Arc::new(GroupRegistry::default());
    let player_registry = Arc::new(PlayerRegistry::with_canonical_player_fixtures_like_cpp());
    let (leader_tx, leader_rx) = bounded::<Vec<u8>>(4);
    let (member_tx, member_rx) = bounded::<Vec<u8>>(4);
    let mut group = GroupInfo::new(leader);
    group.add_member(member);
    group.add_raid_marker_like_cpp(1, 571, Position::xyz(1.0, 2.0, 3.0), ObjectGuid::EMPTY);
    group.add_raid_marker_like_cpp(3, 571, remaining_position, ObjectGuid::EMPTY);
    let group_guid = group.group_guid;
    group_registry.register_group_like_cpp(group_guid, group);
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

    session.set_player_guid(Some(leader));
    session.group_guid = Some(group_guid);
    session.set_group_registry(
        Arc::clone(&group_registry),
        Arc::new(PendingInvites::default()),
    );
    session.set_player_registry(player_registry);

    session
        .handle_clear_raid_marker(clear_raid_marker_packet(1))
        .await;

    assert_eq!(
        group_registry
            .get(&group_guid)
            .unwrap()
            .active_raid_markers_mask_like_cpp(),
        1 << 3
    );
    for rx in [&leader_rx, &member_rx] {
        let bytes = rx.try_recv().expect("marker changed");
        assert_raid_markers_packet_like_cpp(&bytes, 1 << 3, &[remaining_position]);
    }
}
#[tokio::test]
async fn clear_raid_marker_id_eight_removes_all_like_cpp() {
    let (mut session, _send_rx) = make_session_with_send();
    let leader = ObjectGuid::create_player(1, 42);
    let group_registry = Arc::new(GroupRegistry::default());
    let player_registry = Arc::new(PlayerRegistry::with_canonical_player_fixtures_like_cpp());
    let (leader_tx, leader_rx) = bounded::<Vec<u8>>(4);
    let mut group = GroupInfo::new(leader);
    group.add_raid_marker_like_cpp(1, 571, Position::xyz(1.0, 2.0, 3.0), ObjectGuid::EMPTY);
    group.add_raid_marker_like_cpp(3, 571, Position::xyz(4.0, 5.0, 6.0), ObjectGuid::EMPTY);
    let group_guid = group.group_guid;
    group_registry.register_group_like_cpp(group_guid, group);
    player_registry.register_or_replace(
        leader,
        broadcast_info(leader, leader_tx),
        Default::default(),
    );

    session.set_player_guid(Some(leader));
    session.group_guid = Some(group_guid);
    session.set_group_registry(
        Arc::clone(&group_registry),
        Arc::new(PendingInvites::default()),
    );
    session.set_player_registry(player_registry);

    session
        .handle_clear_raid_marker(clear_raid_marker_packet(8))
        .await;

    assert_eq!(
        group_registry
            .get(&group_guid)
            .unwrap()
            .active_raid_markers_mask_like_cpp(),
        0
    );
    let bytes = leader_rx.try_recv().expect("marker changed");
    assert_raid_markers_packet_like_cpp(&bytes, 0, &[]);
}
#[tokio::test]
async fn clear_raid_marker_raid_requires_leader_or_assistant_like_cpp() {
    let (mut session, _send_rx) = make_session_with_send();
    let leader = ObjectGuid::create_player(1, 42);
    let member = ObjectGuid::create_player(1, 43);
    let group_registry = Arc::new(GroupRegistry::default());
    let player_registry = Arc::new(PlayerRegistry::with_canonical_player_fixtures_like_cpp());
    let (member_tx, member_rx) = bounded::<Vec<u8>>(4);
    let mut group = GroupInfo::new(leader);
    group.convert_to_raid_like_cpp();
    group.add_member(member);
    group.add_raid_marker_like_cpp(3, 571, Position::xyz(4.0, 5.0, 6.0), ObjectGuid::EMPTY);
    let group_guid = group.group_guid;
    group_registry.register_group_like_cpp(group_guid, group);
    player_registry.register_or_replace(
        member,
        broadcast_info(member, member_tx),
        Default::default(),
    );

    session.set_player_guid(Some(member));
    session.group_guid = Some(group_guid);
    session.set_group_registry(
        Arc::clone(&group_registry),
        Arc::new(PendingInvites::default()),
    );
    session.set_player_registry(player_registry);

    session
        .handle_clear_raid_marker(clear_raid_marker_packet(3))
        .await;

    assert_eq!(
        group_registry
            .get(&group_guid)
            .unwrap()
            .active_raid_markers_mask_like_cpp(),
        1 << 3
    );
    assert!(member_rx.try_recv().is_err());
}
#[tokio::test]
async fn set_role_without_group_sends_only_caller_and_idempotent_zero_returns_like_cpp() {
    let (mut session, send_rx) = make_session_with_send();
    let sender = ObjectGuid::create_player(1, 42);
    let target = ObjectGuid::create_player(1, 43);
    session.set_player_guid(Some(sender));

    session
        .handle_set_role(set_role_packet(target, 0, None))
        .await;
    assert!(send_rx.try_recv().is_err());

    session
        .handle_set_role(set_role_packet(target, 4, Some(0)))
        .await;

    let sent = send_rx.try_recv().expect("caller role changed inform");
    let mut pkt = WorldPacket::from_bytes(&sent);
    assert_eq!(
        pkt.read_uint16().unwrap(),
        ServerOpcodes::RoleChangedInform as u16
    );
    assert_eq!(pkt.read_uint8().unwrap(), 0);
    assert_eq!(pkt.read_packed_guid().unwrap(), sender);
    assert_eq!(pkt.read_packed_guid().unwrap(), target);
    assert_eq!(pkt.read_uint8().unwrap(), 0);
    assert_eq!(pkt.read_uint8().unwrap(), 4);
    assert!(send_rx.try_recv().is_err());
}
#[tokio::test]
async fn set_role_group_old_equal_returns_without_packet_or_mutation_like_cpp() {
    let (mut session, _send_rx) = make_session_with_send();
    let leader = ObjectGuid::create_player(1, 42);
    let member = ObjectGuid::create_player(1, 43);
    let group_registry = Arc::new(GroupRegistry::default());
    let mut group = GroupInfo::new(leader);
    group.add_member(member);
    group.set_lfg_roles_like_cpp(member, 2);
    let group_guid = group.group_guid;
    group_registry.register_group_like_cpp(group_guid, group);
    let player_registry = Arc::new(PlayerRegistry::with_canonical_player_fixtures_like_cpp());
    let (member_tx, member_rx) = bounded(8);
    player_registry.register_or_replace(
        member,
        broadcast_info(member, member_tx),
        Default::default(),
    );

    session.set_player_guid(Some(leader));
    session.group_guid = Some(group_guid);
    session.set_player_registry(player_registry);
    session.set_group_registry(group_registry.clone(), Arc::new(PendingInvites::default()));

    session
        .handle_set_role(set_role_packet(member, 2, None))
        .await;

    assert_eq!(
        group_registry
            .get(&group_guid)
            .unwrap()
            .get_lfg_roles_like_cpp(member),
        2
    );
    assert!(member_rx.try_recv().is_err());
}
#[tokio::test]
async fn convert_raid_sets_flag_and_queues_member_refresh_like_cpp() {
    let (mut session, send_rx) = make_session_with_send();
    let leader = ObjectGuid::create_player(1, 42);
    let member = ObjectGuid::create_player(1, 43);
    let group_registry = Arc::new(GroupRegistry::default());
    let mut group = GroupInfo::new(leader);
    group.add_member(member);
    let group_guid = group.group_guid;
    group_registry.register_group_like_cpp(group_guid, group);

    let player_registry = Arc::new(PlayerRegistry::with_canonical_player_fixtures_like_cpp());
    let (leader_tx, leader_rx) = bounded(8);
    let (member_tx, _member_rx) = bounded(8);
    let (member_command_tx, member_command_rx) =
        test_session_command_dispatcher(member, member_tx.clone());
    player_registry.register_or_replace(
        leader,
        broadcast_info(leader, leader_tx),
        Default::default(),
    );
    player_registry.register_or_replace(
        member,
        broadcast_info_with_command_tx(member, member_tx, member_command_tx),
        Default::default(),
    );

    session.set_player_guid(Some(leader));
    session.group_guid = Some(group_guid);
    session.set_player_registry(Arc::clone(&player_registry));
    session.set_group_registry(group_registry.clone(), Arc::new(PendingInvites::default()));

    session.handle_convert_raid(convert_raid_packet(true)).await;

    assert!(
        group_registry
            .get(&group_guid)
            .is_some_and(|group| group.is_raid_group())
    );
    let remote_refresh = member_command_rx
        .recv_timeout(Duration::from_secs(1))
        .expect("remote member visible refresh command queued");
    assert!(matches!(
        remote_refresh,
        SessionCommand::RefreshVisibleGameobjectsOrSpellClicksLikeCpp
    ));
    let command_result = send_rx.try_recv().expect("party command result");
    assert_eq!(
        u16::from_le_bytes([command_result[0], command_result[1]]),
        ServerOpcodes::PartyCommandResult as u16
    );
    assert!(send_rx.try_recv().is_err());
    let party_update = recv_dispatched_packet(&leader_rx, "leader party update");
    assert_eq!(
        u16::from_le_bytes([party_update[0], party_update[1]]),
        ServerOpcodes::PartyUpdate as u16
    );
    assert_eq!(
        u16::from_le_bytes([party_update[2], party_update[3]]),
        wow_social::group::GROUP_FLAG_RAID_LIKE_CPP
    );
}
#[tokio::test]
async fn convert_raid_releases_group_guard_before_refresh_backpressure_like_cpp() {
    let (mut session, _send_rx) = make_session_with_send();
    let leader = ObjectGuid::create_player(1, 142);
    let member = ObjectGuid::create_player(1, 143);
    let group_registry = Arc::new(GroupRegistry::default());
    let mut group = GroupInfo::new(leader);
    group.add_member(member);
    let group_guid = group.group_guid;
    group_registry.register_group_like_cpp(group_guid, group);

    let player_registry = Arc::new(PlayerRegistry::with_canonical_player_fixtures_like_cpp());
    let (leader_tx, _leader_rx) = bounded(8);
    player_registry.register_or_replace(
        leader,
        broadcast_info(leader, leader_tx),
        Default::default(),
    );
    let (member_tx, _member_rx) = bounded(8);
    // PartyUpdate fills this single slot; the following async refresh then
    // waits for its timeout and exposes any DashMap guard held across it.
    let (member_command_tx, _member_command_rx) = flume::bounded(1);
    player_registry.register_or_replace(
        member,
        broadcast_info_with_command_tx(member, member_tx, member_command_tx),
        Default::default(),
    );

    session.set_player_guid(Some(leader));
    session.group_guid = Some(group_guid);
    session.set_player_registry(player_registry);
    session.set_group_registry(
        Arc::clone(&group_registry),
        Arc::new(PendingInvites::default()),
    );

    let mut conversion = Box::pin(session.handle_convert_raid(convert_raid_packet(true)));
    tokio::select! {
        () = &mut conversion => panic!("refresh should be waiting on the full command channel"),
        () = tokio::time::sleep(Duration::from_millis(20)) => {}
    }

    let writer_registry = Arc::clone(&group_registry);
    let writer = tokio::task::spawn_blocking(move || {
        writer_registry
            .set_everyone_assistant_transition_like_cpp(group_guid, leader, true)
            .ok()
            .map(|outcome| outcome.group.group_flags)
    });
    let observed_flags = tokio::time::timeout(Duration::from_secs(1), writer)
        .await
        .expect("the group write must not wait for refresh channel backpressure")
        .expect("group writer task should not panic")
        .expect("converted group should remain registered");
    assert_ne!(
        observed_flags & wow_social::group::GROUP_FLAG_RAID_LIKE_CPP,
        0
    );

    conversion.await;
}
#[tokio::test]
async fn convert_raid_to_group_rejects_over_five_members_like_cpp() {
    let (mut session, _send_rx) = make_session_with_send();
    let leader = ObjectGuid::create_player(1, 42);
    let group_registry = Arc::new(GroupRegistry::default());
    let mut group = GroupInfo::new(leader);
    for counter in 43..48 {
        group.add_member(ObjectGuid::create_player(1, counter));
    }
    group.convert_to_raid_like_cpp();
    let group_guid = group.group_guid;
    group_registry.register_group_like_cpp(group_guid, group);

    let player_registry = Arc::new(PlayerRegistry::with_canonical_player_fixtures_like_cpp());
    session.set_player_guid(Some(leader));
    session.group_guid = Some(group_guid);
    session.set_player_registry(player_registry);
    session.set_group_registry(group_registry.clone(), Arc::new(PendingInvites::default()));

    session
        .handle_convert_raid(convert_raid_packet(false))
        .await;

    assert!(
        group_registry
            .get(&group_guid)
            .is_some_and(|group| group.is_raid_group())
    );
}
#[tokio::test]
async fn change_sub_group_leader_moves_member_and_fans_out_update_like_cpp() {
    let (mut session, send_rx) = make_session_with_send();
    let leader = ObjectGuid::create_player(1, 42);
    let member = ObjectGuid::create_player(1, 43);
    let group_registry = Arc::new(GroupRegistry::default());
    let mut group = GroupInfo::new(leader);
    group.add_member(member);
    group.convert_to_raid_like_cpp();
    let group_guid = group.group_guid;
    group_registry.register_group_like_cpp(group_guid, group);

    let player_registry = Arc::new(PlayerRegistry::with_canonical_player_fixtures_like_cpp());
    let (leader_tx, leader_rx) = bounded(8);
    let (member_tx, member_rx) = bounded(8);
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

    session.set_player_guid(Some(leader));
    session.group_guid = Some(group_guid);
    session.set_player_registry(Arc::clone(&player_registry));
    session.set_group_registry(group_registry.clone(), Arc::new(PendingInvites::default()));

    session
        .handle_change_sub_group(change_sub_group_packet(member, 2, Some(0)))
        .await;

    assert!(send_rx.try_recv().is_err());
    let group = group_registry.get(&group_guid).unwrap();
    assert_eq!(group.member_group_like_cpp(member), 2);
    assert!(group.has_free_slot_sub_group_like_cpp(0));
    let leader_update = recv_dispatched_packet(&leader_rx, "leader party update");
    assert_eq!(
        u16::from_le_bytes([leader_update[0], leader_update[1]]),
        ServerOpcodes::PartyUpdate as u16
    );
    let member_update = recv_dispatched_packet(&member_rx, "member party update");
    assert_eq!(
        u16::from_le_bytes([member_update[0], member_update[1]]),
        ServerOpcodes::PartyUpdate as u16
    );
}
#[tokio::test]
async fn change_sub_group_assistant_allowed_but_regular_member_rejected_like_cpp() {
    let (mut session, _send_rx) = make_session_with_send();
    let leader = ObjectGuid::create_player(1, 42);
    let assistant = ObjectGuid::create_player(1, 43);
    let target = ObjectGuid::create_player(1, 44);
    let group_registry = Arc::new(GroupRegistry::default());
    let mut group = GroupInfo::new(leader);
    group.add_member(assistant);
    group.add_member(target);
    group.convert_to_raid_like_cpp();
    let group_guid = group.group_guid;
    group_registry.register_group_like_cpp(group_guid, group);

    let player_registry = Arc::new(PlayerRegistry::with_canonical_player_fixtures_like_cpp());
    let (leader_tx, _leader_rx) = bounded(8);
    let (assistant_tx, _assistant_rx) = bounded(8);
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
        .handle_change_sub_group(change_sub_group_packet(target, 2, None))
        .await;
    assert_eq!(
        group_registry
            .get(&group_guid)
            .unwrap()
            .member_group_like_cpp(target),
        0
    );

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
        .handle_change_sub_group(change_sub_group_packet(target, 2, None))
        .await;

    assert_eq!(
        group_registry
            .get(&group_guid)
            .unwrap()
            .member_group_like_cpp(target),
        2
    );
}
#[tokio::test]
async fn set_party_assignment_leader_sets_main_tank_and_fans_out_like_cpp() {
    let (mut session, send_rx) = make_session_with_send();
    let leader = ObjectGuid::create_player(1, 42);
    let member = ObjectGuid::create_player(1, 43);
    let group_registry = Arc::new(GroupRegistry::default());
    let mut group = GroupInfo::new(leader);
    group.add_member(member);
    group.convert_to_raid_like_cpp();
    let group_guid = group.group_guid;
    group_registry.register_group_like_cpp(group_guid, group);

    let player_registry = Arc::new(PlayerRegistry::with_canonical_player_fixtures_like_cpp());
    let (leader_tx, leader_rx) = bounded(8);
    let (member_tx, member_rx) = bounded(8);
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

    session.set_player_guid(Some(leader));
    session.group_guid = Some(group_guid);
    session.set_player_registry(Arc::clone(&player_registry));
    session.set_group_registry(group_registry.clone(), Arc::new(PendingInvites::default()));

    session
        .handle_set_party_assignment(set_party_assignment_packet(
            wow_social::group::GROUP_ASSIGN_MAINTANK_LIKE_CPP,
            member,
            true,
            Some(0),
        ))
        .await;

    assert!(send_rx.try_recv().is_err());
    assert_eq!(
        group_registry
            .get(&group_guid)
            .unwrap()
            .member_slot_like_cpp(member)
            .unwrap()
            .flags
            & wow_social::group::MEMBER_FLAG_MAINTANK_LIKE_CPP,
        wow_social::group::MEMBER_FLAG_MAINTANK_LIKE_CPP
    );
    let leader_update = recv_dispatched_packet(&leader_rx, "leader party update");
    assert_eq!(
        u16::from_le_bytes([leader_update[0], leader_update[1]]),
        ServerOpcodes::PartyUpdate as u16
    );
    let member_update = recv_dispatched_packet(&member_rx, "member party update");
    assert_eq!(
        u16::from_le_bytes([member_update[0], member_update[1]]),
        ServerOpcodes::PartyUpdate as u16
    );
}
#[tokio::test]
async fn set_party_assignment_assistant_sets_main_assist_like_cpp() {
    let (mut session, _send_rx) = make_session_with_send();
    let leader = ObjectGuid::create_player(1, 42);
    let assistant = ObjectGuid::create_player(1, 43);
    let target = ObjectGuid::create_player(1, 44);
    let group_registry = Arc::new(GroupRegistry::default());
    let mut group = GroupInfo::new(leader);
    group.add_member(assistant);
    group.add_member(target);
    group.convert_to_raid_like_cpp();
    group
        .set_group_member_flag_like_cpp(
            assistant,
            true,
            wow_social::group::MEMBER_FLAG_ASSISTANT_LIKE_CPP,
        )
        .unwrap();
    let group_guid = group.group_guid;
    group_registry.register_group_like_cpp(group_guid, group);

    let player_registry = Arc::new(PlayerRegistry::with_canonical_player_fixtures_like_cpp());
    let (leader_tx, leader_rx) = bounded(8);
    let (assistant_tx, _assistant_rx) = bounded(8);
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
        .handle_set_party_assignment(set_party_assignment_packet(
            wow_social::group::GROUP_ASSIGN_MAINASSIST_LIKE_CPP,
            target,
            true,
            None,
        ))
        .await;

    assert_eq!(
        group_registry
            .get(&group_guid)
            .unwrap()
            .member_slot_like_cpp(target)
            .unwrap()
            .flags
            & wow_social::group::MEMBER_FLAG_MAINASSIST_LIKE_CPP,
        wow_social::group::MEMBER_FLAG_MAINASSIST_LIKE_CPP
    );
    let _ = recv_dispatched_packet(&leader_rx, "leader party update");
}
#[tokio::test]
async fn set_party_assignment_rejects_regular_member_without_mutation_or_fanout_like_cpp() {
    let (mut session, _send_rx) = make_session_with_send();
    let leader = ObjectGuid::create_player(1, 42);
    let member = ObjectGuid::create_player(1, 43);
    let target = ObjectGuid::create_player(1, 44);
    let group_registry = Arc::new(GroupRegistry::default());
    let mut group = GroupInfo::new(leader);
    group.add_member(member);
    group.add_member(target);
    group.convert_to_raid_like_cpp();
    let group_guid = group.group_guid;
    group_registry.register_group_like_cpp(group_guid, group);

    let player_registry = Arc::new(PlayerRegistry::with_canonical_player_fixtures_like_cpp());
    let (leader_tx, leader_rx) = bounded(8);
    let (member_tx, member_rx) = bounded(8);
    let (target_tx, target_rx) = bounded(8);
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
    session.set_player_registry(Arc::clone(&player_registry));
    session.set_group_registry(group_registry.clone(), Arc::new(PendingInvites::default()));

    session
        .handle_set_party_assignment(set_party_assignment_packet(
            wow_social::group::GROUP_ASSIGN_MAINTANK_LIKE_CPP,
            target,
            true,
            None,
        ))
        .await;

    assert_eq!(
        group_registry
            .get(&group_guid)
            .unwrap()
            .member_slot_like_cpp(target)
            .unwrap()
            .flags,
        0
    );
    assert!(leader_rx.try_recv().is_err());
    assert!(member_rx.try_recv().is_err());
    assert!(target_rx.try_recv().is_err());
}
#[tokio::test]
async fn set_party_assignment_non_raid_or_missing_target_fans_out_and_missing_clears_unique_like_cpp()
 {
    let (mut session, _send_rx) = make_session_with_send();
    let leader = ObjectGuid::create_player(1, 42);
    let member = ObjectGuid::create_player(1, 43);
    let missing = ObjectGuid::create_player(1, 44);
    let group_registry = Arc::new(GroupRegistry::default());
    let mut group = GroupInfo::new(leader);
    group.add_member(member);
    let group_guid = group.group_guid;
    group_registry.register_group_like_cpp(group_guid, group);

    let player_registry = Arc::new(PlayerRegistry::with_canonical_player_fixtures_like_cpp());
    let (leader_tx, leader_rx) = bounded(8);
    let (member_tx, member_rx) = bounded(8);
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

    session.set_player_guid(Some(leader));
    session.group_guid = Some(group_guid);
    session.set_player_registry(Arc::clone(&player_registry));
    session.set_group_registry(group_registry.clone(), Arc::new(PendingInvites::default()));

    session
        .handle_set_party_assignment(set_party_assignment_packet(
            wow_social::group::GROUP_ASSIGN_MAINTANK_LIKE_CPP,
            member,
            true,
            None,
        ))
        .await;
    assert_eq!(
        group_registry
            .get(&group_guid)
            .unwrap()
            .member_slot_like_cpp(member)
            .unwrap()
            .flags,
        0
    );
    let _ = recv_dispatched_packet(&leader_rx, "leader party update");
    let _ = recv_dispatched_packet(&member_rx, "member party update");

    group_registry
        .convert_group_like_cpp(group_guid, leader, true)
        .unwrap();
    group_registry
        .set_member_flag_transition_like_cpp(
            group_guid,
            leader,
            member,
            true,
            wow_social::group::MEMBER_FLAG_MAINTANK_LIKE_CPP,
        )
        .unwrap();
    session
        .handle_set_party_assignment(set_party_assignment_packet(
            wow_social::group::GROUP_ASSIGN_MAINTANK_LIKE_CPP,
            missing,
            true,
            None,
        ))
        .await;
    assert_eq!(
        group_registry
            .get(&group_guid)
            .unwrap()
            .member_slot_like_cpp(member)
            .unwrap()
            .flags
            & wow_social::group::MEMBER_FLAG_MAINTANK_LIKE_CPP,
        0
    );
    let _ = recv_dispatched_packet(&leader_rx, "leader party update");
    let _ = recv_dispatched_packet(&member_rx, "member party update");
}
#[tokio::test]
async fn set_party_assignment_unknown_assignment_fans_out_without_mutation_like_cpp() {
    let (mut session, _send_rx) = make_session_with_send();
    let leader = ObjectGuid::create_player(1, 42);
    let member = ObjectGuid::create_player(1, 43);
    let group_registry = Arc::new(GroupRegistry::default());
    let mut group = GroupInfo::new(leader);
    group.add_member(member);
    group.convert_to_raid_like_cpp();
    let sequence_before = group.sequence_num;
    let group_guid = group.group_guid;
    group_registry.register_group_like_cpp(group_guid, group);

    let player_registry = Arc::new(PlayerRegistry::with_canonical_player_fixtures_like_cpp());
    let (leader_tx, leader_rx) = bounded(8);
    let (member_tx, member_rx) = bounded(8);
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

    session.set_player_guid(Some(leader));
    session.group_guid = Some(group_guid);
    session.set_player_registry(Arc::clone(&player_registry));
    session.set_group_registry(group_registry.clone(), Arc::new(PendingInvites::default()));

    session
        .handle_set_party_assignment(set_party_assignment_packet(99, member, true, None))
        .await;

    let group = group_registry.get(&group_guid).unwrap();
    assert_eq!(group.sequence_num, sequence_before);
    assert_eq!(group.member_slot_like_cpp(member).unwrap().flags, 0);
    let _ = recv_dispatched_packet(&leader_rx, "leader party update");
    let _ = recv_dispatched_packet(&member_rx, "member party update");
}
